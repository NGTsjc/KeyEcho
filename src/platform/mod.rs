use std::collections::HashSet;

use anyhow::Result;

use crate::config::HotkeySpec;
use crate::language::UiText;

#[derive(Debug, Clone)]
pub struct HotkeyRegistration {
    pub id: String,
    pub hotkey: HotkeySpec,
}

#[derive(Debug, Clone)]
pub enum PlatformEvent {
    OverlapChanged(bool),
    RestartRequested,
    ExitRequested,
    HotkeyTriggered(String),
}

pub trait PlatformRuntime {
    fn set_allow_overlap(&mut self, enabled: bool) -> Result<()>;
    fn set_ui_text(&mut self, text: &UiText) -> Result<()>;
    fn register_hotkeys(&mut self, bindings: &[HotkeyRegistration]) -> Result<HashSet<String>>;
    fn clear_hotkeys(&mut self) -> Result<()>;
    fn wait_for_event(&mut self) -> Result<PlatformEvent>;
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub fn create_runtime() -> Result<Box<dyn PlatformRuntime>> {
    Ok(Box::new(windows::WindowsPlatformRuntime::new()?))
}

#[cfg(target_os = "windows")]
pub fn is_another_instance_running() -> bool {
    windows::is_another_instance_running()
}

#[cfg(not(target_os = "windows"))]
pub fn create_runtime() -> Result<Box<dyn PlatformRuntime>> {
    anyhow::bail!("this build currently supports Windows only")
}

#[cfg(not(target_os = "windows"))]
pub fn is_another_instance_running() -> bool {
    false
}
