use std::collections::HashSet;

use anyhow::Result;

use crate::config::HotkeySpec;

#[derive(Debug, Clone)]
pub struct HotkeyRegistration {
    pub id: String,
    pub hotkey: HotkeySpec,
}

#[derive(Debug, Clone)]
pub enum PlatformEvent {
    RestartRequested,
    ExitRequested,
    HotkeyTriggered(String),
}

pub trait PlatformRuntime {
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
