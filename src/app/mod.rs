use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{error, info, warn};
use simplelog::{ConfigBuilder, LevelFilter, SimpleLogger, WriteLogger};

use crate::audio::{self, AudioPlayer};
use crate::config::{self, BindingConfig};
use crate::platform::{self, HotkeyRegistration, PlatformEvent, PlatformRuntime};

pub fn run() -> Result<()> {
    let config_path = config::default_config_path()?;
    if platform::is_another_instance_running() {
        record_duplicate_launch(&config_path);
        return Ok(());
    }
    initialize_logging(&config_path);
    info!("KeyEcho starting with config {}", config_path.display());
    let runtime = platform::create_runtime()?;
    let audio_player = audio::create_player()?;
    let mut app = AppController::new(config_path, runtime, audio_player);
    app.run()
}

/// 已有实例运行时，不初始化日志（避免截断正在运行的实例的日志文件），
/// 仅追加一行说明并退出。
fn record_duplicate_launch(config_path: &Path) {
    use std::io::Write;
    let log_path = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("keyecho.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(&log_path) {
        let _ = file.write_all(b"Another KeyEcho instance is already running, exiting\n");
    }
    eprintln!("Another KeyEcho instance is already running, exiting");
}

fn initialize_logging(config_path: &Path) {
    let log_path = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("keyecho.log");
    let logger_config = ConfigBuilder::new().set_time_format_rfc3339().build();
    match File::create(&log_path) {
        Ok(file) => {
            if WriteLogger::init(LevelFilter::Info, logger_config, file).is_ok() {
                return;
            }
        }
        Err(error) => {
            eprintln!("Failed to create log file {}: {error}", log_path.display());
        }
    }
    let _ = SimpleLogger::init(LevelFilter::Info, ConfigBuilder::new().build());
}

struct AppController {
    config_path: PathBuf,
    runtime: Box<dyn PlatformRuntime>,
    audio_player: Box<dyn AudioPlayer>,
    active_bindings: HashMap<String, BindingConfig>,
}

impl AppController {
    fn new(
        config_path: PathBuf,
        runtime: Box<dyn PlatformRuntime>,
        audio_player: Box<dyn AudioPlayer>,
    ) -> Self {
        Self {
            config_path,
            runtime,
            audio_player,
            active_bindings: HashMap::new(),
        }
    }

    fn run(&mut self) -> Result<()> {
        self.reload_bindings()?;
        loop {
            let event = self.runtime.wait_for_event()?;
            if !self.process_event(event)? {
                return Ok(());
            }
        }
    }

    fn process_event(&mut self, event: PlatformEvent) -> Result<bool> {
        match event {
            PlatformEvent::RestartRequested => {
                info!("Tray requested configuration reload");
                self.reload_bindings()?;
                Ok(true)
            }
            PlatformEvent::ExitRequested => {
                info!("Tray requested shutdown");
                self.runtime.clear_hotkeys()?;
                Ok(false)
            }
            PlatformEvent::HotkeyTriggered(binding_id) => {
                self.handle_hotkey(&binding_id);
                Ok(true)
            }
        }
    }

    fn reload_bindings(&mut self) -> Result<()> {
        self.runtime.clear_hotkeys()?;
        let bindings = match config::load_config(&self.config_path) {
            Ok(bindings) => bindings,
            Err(error) => {
                error!(
                    "Failed to load config {}: {error:#}",
                    self.config_path.display()
                );
                self.active_bindings.clear();
                return Ok(());
            }
        };
        let registrations = bindings
            .iter()
            .map(|binding| HotkeyRegistration {
                id: binding.id.clone(),
                hotkey: binding.hotkey.clone(),
            })
            .collect::<Vec<_>>();
        let registered_ids = self.runtime.register_hotkeys(&registrations)?;
        self.active_bindings = retain_registered_bindings(bindings, &registered_ids);
        info!(
            "Active hotkeys after reload: {}",
            self.active_bindings.len()
        );
        Ok(())
    }

    fn handle_hotkey(&self, binding_id: &str) {
        let Some(binding) = self.active_bindings.get(binding_id) else {
            warn!("Received hotkey event for unknown binding id `{binding_id}`");
            return;
        };
        if let Err(error) = self.audio_player.play(&binding.audio_path) {
            error!(
                "Failed to play binding `{}` from {}: {error:#}",
                binding.id,
                binding.audio_path.display()
            );
        } else {
            info!(
                "Playback started for binding `{}` from {}",
                binding.id,
                binding.audio_path.display()
            );
        }
    }
}

fn retain_registered_bindings(
    bindings: Vec<BindingConfig>,
    registered_ids: &HashSet<String>,
) -> HashMap<String, BindingConfig> {
    bindings
        .into_iter()
        .filter(|binding| registered_ids.contains(&binding.id))
        .map(|binding| (binding.id.clone(), binding))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use anyhow::{Result, anyhow};
    use tempfile::tempdir;

    use super::AppController;
    use crate::audio::AudioPlayer;
    use crate::platform::{HotkeyRegistration, PlatformEvent, PlatformRuntime};

    #[test]
    fn restart_reloads_updated_config() {
        let temp_dir = tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("config.json");
        write_config(&config_path, "first", "CTRL+ALT+1", "asset/one.wav");
        let runtime_state = Arc::new(Mutex::new(MockRuntimeState::default()));
        let audio_state = Arc::new(Mutex::new(MockAudioState::default()));
        let runtime = Box::new(MockRuntime::new(runtime_state.clone()));
        let audio = Box::new(MockAudioPlayer::new(audio_state));
        let mut controller = AppController::new(config_path.clone(), runtime, audio);
        controller
            .reload_bindings()
            .expect("initial reload should work");
        write_config(&config_path, "second", "CTRL+ALT+2", "asset/two.wav");
        controller
            .process_event(PlatformEvent::RestartRequested)
            .expect("restart should reload config");
        let state = runtime_state.lock().expect("runtime state should lock");
        assert_eq!(state.clear_hotkeys_calls, 2);
        assert_eq!(state.registered_batches.len(), 2);
        assert_eq!(state.registered_batches[1][0].id, "second");
        assert_eq!(controller.active_bindings.len(), 1);
        assert!(controller.active_bindings.contains_key("second"));
    }

    #[test]
    fn hotkey_event_plays_bound_audio() {
        let temp_dir = tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("config.json");
        write_config(&config_path, "sample", "CTRL+ALT+1", "asset/sample.wav");
        let runtime = Box::new(MockRuntime::new(Arc::new(Mutex::new(
            MockRuntimeState::default(),
        ))));
        let audio_state = Arc::new(Mutex::new(MockAudioState::default()));
        let audio = Box::new(MockAudioPlayer::new(audio_state.clone()));
        let mut controller = AppController::new(config_path.clone(), runtime, audio);
        controller.reload_bindings().expect("reload should work");
        controller
            .process_event(PlatformEvent::HotkeyTriggered("sample".to_owned()))
            .expect("hotkey should be processed");
        let state = audio_state.lock().expect("audio state should lock");
        assert_eq!(
            state.played_paths,
            vec![temp_dir.path().join("asset").join("sample.wav")]
        );
    }

    #[test]
    fn exit_event_clears_hotkeys_and_stops_loop() {
        let temp_dir = tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("config.json");
        write_config(&config_path, "sample", "CTRL+ALT+1", "asset/sample.wav");
        let runtime_state = Arc::new(Mutex::new(MockRuntimeState::default()));
        let runtime = Box::new(MockRuntime::new(runtime_state.clone()));
        let audio = Box::new(MockAudioPlayer::new(Arc::new(Mutex::new(
            MockAudioState::default(),
        ))));
        let mut controller = AppController::new(config_path, runtime, audio);
        controller.reload_bindings().expect("reload should work");
        let should_continue = controller
            .process_event(PlatformEvent::ExitRequested)
            .expect("exit should be processed");
        assert!(!should_continue);
        let state = runtime_state.lock().expect("runtime state should lock");
        assert_eq!(state.clear_hotkeys_calls, 2);
    }

    fn write_config(path: &Path, id: &str, hotkey: &str, audio_path: &str) {
        let content = format!(
            "{{\n  \"bindings\": [\n    {{ \"id\": \"{id}\", \"hotkey\": \"{hotkey}\", \"audio_path\": \"{audio_path}\" }}\n  ]\n}}\n"
        );
        fs::write(path, content).expect("config should be written");
    }

    #[derive(Default)]
    struct MockRuntimeState {
        clear_hotkeys_calls: usize,
        registered_batches: Vec<Vec<HotkeyRegistration>>,
    }

    struct MockRuntime {
        state: Arc<Mutex<MockRuntimeState>>,
        queued_events: VecDeque<PlatformEvent>,
    }

    impl MockRuntime {
        fn new(state: Arc<Mutex<MockRuntimeState>>) -> Self {
            Self {
                state,
                queued_events: VecDeque::new(),
            }
        }
    }

    impl PlatformRuntime for MockRuntime {
        fn register_hotkeys(&mut self, bindings: &[HotkeyRegistration]) -> Result<HashSet<String>> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("runtime state poisoned"))?;
            state.registered_batches.push(bindings.to_vec());
            Ok(bindings.iter().map(|binding| binding.id.clone()).collect())
        }

        fn clear_hotkeys(&mut self) -> Result<()> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("runtime state poisoned"))?;
            state.clear_hotkeys_calls += 1;
            Ok(())
        }

        fn wait_for_event(&mut self) -> Result<PlatformEvent> {
            self.queued_events
                .pop_front()
                .ok_or_else(|| anyhow!("no queued event"))
        }
    }

    #[derive(Default)]
    struct MockAudioState {
        played_paths: Vec<PathBuf>,
    }

    struct MockAudioPlayer {
        state: Arc<Mutex<MockAudioState>>,
    }

    impl MockAudioPlayer {
        fn new(state: Arc<Mutex<MockAudioState>>) -> Self {
            Self { state }
        }
    }

    impl AudioPlayer for MockAudioPlayer {
        fn play(&self, path: &Path) -> Result<()> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow!("audio state poisoned"))?;
            state.played_paths.push(path.to_path_buf());
            Ok(())
        }
    }
}
