use std::collections::HashSet;
use std::mem::MaybeUninit;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, anyhow};
use crossbeam_channel::{Receiver, Sender, bounded};
use log::info;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5,
    VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17,
    VK_F18, VK_F19, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_HOME, VK_INSERT, VK_LCONTROL,
    VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_OEM_1, VK_OEM_2, VK_OEM_3,
    VK_OEM_5, VK_OEM_7, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PRIOR,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
    VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PM_REMOVE, PeekMessageW,
    SetWindowsHookExW, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::config::KeyCode;
use crate::language::UiText;
use crate::platform::{HotkeyRegistration, PlatformEvent, PlatformRuntime};

static HOOK_STATE: OnceLock<Mutex<HookState>> = OnceLock::new();

const SINGLE_INSTANCE_MUTEX_NAME: &str = "KeyEcho.SingleInstance";

/// 通过命名互斥体检测是否已有另一实例在运行。
/// 首个实例创建互斥体后句柄保持打开（进程退出时自动释放）；
/// 后续实例创建同名互斥体时返回 `ERROR_ALREADY_EXISTS`。
pub fn is_another_instance_running() -> bool {
    let wide_name: Vec<u16> = SINGLE_INSTANCE_MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, wide_name.as_ptr());
        if handle.is_null() {
            return false;
        }
        // HANDLE 是裸指针，局部变量析构不会关闭句柄，互斥体存活到进程退出。
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

struct HookState {
    pressed_keys: HashSet<u32>,
    chord_bindings: Vec<(HashSet<u32>, String)>,
    triggered_chords: HashSet<String>,
    event_sender: Sender<PlatformEvent>,
}

pub struct WindowsPlatformRuntime {
    _tray_icon: TrayIcon,
    _menu: Menu,
    restart_menu_id: MenuId,
    exit_menu_id: MenuId,
    restart_item: MenuItem,
    exit_item: MenuItem,
    overlap_item: CheckMenuItem,
    menu_receiver: Receiver<MenuEvent>,
    hook_event_receiver: Receiver<PlatformEvent>,
}

impl WindowsPlatformRuntime {
    pub fn new() -> Result<Self> {
        ensure_windows_message_queue();
        let menu = Menu::new();
        let text = UiText::default();
        let restart_item = MenuItem::new(&text.tray_reload_config, true, None);
        let exit_item = MenuItem::new(&text.tray_exit, true, None);
        let overlap_item = CheckMenuItem::new(&text.tray_allow_overlap, true, false, None);
        menu.append(&overlap_item)
            .context("failed to add overlap tray menu item")?;
        menu.append(&restart_item)
            .context("failed to add restart tray menu item")?;
        menu.append(&exit_item)
            .context("failed to add exit tray menu item")?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("KeyEcho")
            .with_icon(build_tray_icon()?)
            .with_menu(Box::new(menu.clone()))
            .build()
            .context("failed to create tray icon")?;
        let (event_sender, event_receiver) = bounded::<PlatformEvent>(64);
        let hook_state = HookState {
            pressed_keys: HashSet::new(),
            chord_bindings: Vec::new(),
            triggered_chords: HashSet::new(),
            event_sender,
        };
        HOOK_STATE
            .set(Mutex::new(hook_state))
            .map_err(|_| anyhow!("hook state already initialized"))?;
        let hook_handle = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                std::ptr::null_mut(),
                0,
            )
        };
        if hook_handle.is_null() {
            return Err(anyhow!("failed to install keyboard hook"));
        }
        Ok(Self {
            _tray_icon: tray_icon,
            _menu: menu,
            restart_menu_id: restart_item.id().clone(),
            exit_menu_id: exit_item.id().clone(),
            restart_item,
            exit_item,
            overlap_item,
            menu_receiver: MenuEvent::receiver().clone(),
            hook_event_receiver: event_receiver,
        })
    }

    fn update_chord_bindings(&self, bindings: &[HotkeyRegistration]) {
        let chords: Vec<(HashSet<u32>, String)> = bindings
            .iter()
            .map(|b| {
                let vk_set: HashSet<u32> =
                    b.hotkey.keys.iter().map(|k| key_code_to_vk(*k)).collect();
                (vk_set, b.id.clone())
            })
            .collect();
        if let Some(lock) = HOOK_STATE.get()
            && let Ok(mut state) = lock.lock()
        {
            state.chord_bindings = chords;
        }
    }

    fn clear_chord_bindings(&self) {
        if let Some(lock) = HOOK_STATE.get()
            && let Ok(mut state) = lock.lock()
        {
            state.chord_bindings.clear();
        }
    }
}

impl PlatformRuntime for WindowsPlatformRuntime {
    fn set_allow_overlap(&mut self, enabled: bool) -> Result<()> {
        self.overlap_item.set_checked(enabled);
        Ok(())
    }

    fn set_ui_text(&mut self, text: &UiText) -> Result<()> {
        set_tray_menu_text(
            &self.restart_item,
            &self.exit_item,
            &self.overlap_item,
            text,
        );
        Ok(())
    }

    fn register_hotkeys(&mut self, bindings: &[HotkeyRegistration]) -> Result<HashSet<String>> {
        self.update_chord_bindings(bindings);
        let active_ids: HashSet<String> = bindings.iter().map(|b| b.id.clone()).collect();
        Ok(active_ids)
    }

    fn clear_hotkeys(&mut self) -> Result<()> {
        self.clear_chord_bindings();
        Ok(())
    }

    fn wait_for_event(&mut self) -> Result<PlatformEvent> {
        loop {
            if let Ok(event) = self.menu_receiver.try_recv() {
                if event.id == *self.overlap_item.id() {
                    return Ok(PlatformEvent::OverlapChanged(
                        self.overlap_item.is_checked(),
                    ));
                }
                let is_restart = event.id == self.restart_menu_id;
                let is_exit = event.id == self.exit_menu_id;
                if is_restart {
                    return Ok(PlatformEvent::RestartRequested);
                }
                if is_exit {
                    return Ok(PlatformEvent::ExitRequested);
                }
            }
            if let Ok(event) = self.hook_event_receiver.try_recv() {
                return Ok(event);
            }
            let mut message = MaybeUninit::zeroed();
            let has_message = unsafe {
                PeekMessageW(message.as_mut_ptr(), std::ptr::null_mut(), 0, 0, PM_REMOVE)
            };
            if has_message == 0 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            let message = unsafe { message.assume_init() };
            unsafe {
                DispatchMessageW(&message);
            }
        }
    }
}

impl Drop for WindowsPlatformRuntime {
    fn drop(&mut self) {
        self.clear_chord_bindings();
    }
}

extern "system" fn keyboard_hook_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) };
    }
    let Some(lock) = HOOK_STATE.get() else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) };
    };
    let Ok(mut state) = lock.lock() else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) };
    };
    if state.chord_bindings.is_empty() {
        drop(state);
        return unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) };
    }
    let event = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
    let vk_code = normalize_vk(event.vkCode);
    let is_key_down = w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN;
    let is_key_up = w_param as u32 == WM_KEYUP || w_param as u32 == WM_SYSKEYUP;
    if is_key_down {
        state.pressed_keys.insert(vk_code);
        let mut triggered = Vec::new();
        for (chord, id) in &state.chord_bindings {
            if chord.iter().all(|k| state.pressed_keys.contains(k))
                && !state.triggered_chords.contains(id)
            {
                triggered.push(id.clone());
            }
        }
        for id in triggered {
            state.triggered_chords.insert(id.clone());
            let _ = state
                .event_sender
                .try_send(PlatformEvent::HotkeyTriggered(id));
        }
    } else if is_key_up {
        state.pressed_keys.remove(&vk_code);
        state.triggered_chords.clear();
    }
    drop(state);
    unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) }
}

fn key_code_to_vk(key: KeyCode) -> u32 {
    match key {
        KeyCode::Control => VK_CONTROL as u32,
        KeyCode::Alt => VK_MENU as u32,
        KeyCode::Shift => VK_SHIFT as u32,
        KeyCode::Super => VK_LWIN as u32,
        KeyCode::Character(c) => c.to_ascii_uppercase() as u32,
        KeyCode::Digit(0) => 0x30,
        KeyCode::Digit(1) => 0x31,
        KeyCode::Digit(2) => 0x32,
        KeyCode::Digit(3) => 0x33,
        KeyCode::Digit(4) => 0x34,
        KeyCode::Digit(5) => 0x35,
        KeyCode::Digit(6) => 0x36,
        KeyCode::Digit(7) => 0x37,
        KeyCode::Digit(8) => 0x38,
        KeyCode::Digit(9) => 0x39,
        KeyCode::F(1) => VK_F1 as u32,
        KeyCode::F(2) => VK_F2 as u32,
        KeyCode::F(3) => VK_F3 as u32,
        KeyCode::F(4) => VK_F4 as u32,
        KeyCode::F(5) => VK_F5 as u32,
        KeyCode::F(6) => VK_F6 as u32,
        KeyCode::F(7) => VK_F7 as u32,
        KeyCode::F(8) => VK_F8 as u32,
        KeyCode::F(9) => VK_F9 as u32,
        KeyCode::F(10) => VK_F10 as u32,
        KeyCode::F(11) => VK_F11 as u32,
        KeyCode::F(12) => VK_F12 as u32,
        KeyCode::F(13) => VK_F13 as u32,
        KeyCode::F(14) => VK_F14 as u32,
        KeyCode::F(15) => VK_F15 as u32,
        KeyCode::F(16) => VK_F16 as u32,
        KeyCode::F(17) => VK_F17 as u32,
        KeyCode::F(18) => VK_F18 as u32,
        KeyCode::F(19) => VK_F19 as u32,
        KeyCode::F(20) => VK_F20 as u32,
        KeyCode::F(21) => VK_F21 as u32,
        KeyCode::F(22) => VK_F22 as u32,
        KeyCode::F(23) => VK_F23 as u32,
        KeyCode::F(24) => VK_F24 as u32,
        KeyCode::Space => VK_SPACE as u32,
        KeyCode::Enter => VK_RETURN as u32,
        KeyCode::Tab => VK_TAB as u32,
        KeyCode::Escape => VK_ESCAPE as u32,
        KeyCode::Backspace => VK_BACK as u32,
        KeyCode::Delete => VK_DELETE as u32,
        KeyCode::Insert => VK_INSERT as u32,
        KeyCode::Home => VK_HOME as u32,
        KeyCode::End => VK_END as u32,
        KeyCode::PageUp => VK_PRIOR as u32,
        KeyCode::PageDown => VK_NEXT as u32,
        KeyCode::Up => VK_UP as u32,
        KeyCode::Down => VK_DOWN as u32,
        KeyCode::Left => VK_LEFT as u32,
        KeyCode::Right => VK_RIGHT as u32,
        KeyCode::Minus => VK_OEM_MINUS as u32,
        KeyCode::Equal => VK_OEM_PLUS as u32,
        KeyCode::Comma => VK_OEM_COMMA as u32,
        KeyCode::Period => VK_OEM_PERIOD as u32,
        KeyCode::Slash => VK_OEM_2 as u32,
        KeyCode::Backslash => VK_OEM_5 as u32,
        KeyCode::Semicolon => VK_OEM_1 as u32,
        KeyCode::Quote => VK_OEM_7 as u32,
        KeyCode::Backquote => VK_OEM_3 as u32,
        KeyCode::F(_) | KeyCode::Digit(_) => 0,
    }
}

/// 将物理键盘产生的左右区分 VK 码归一化为通用 VK 码，
/// 确保与 `key_code_to_vk` 生成的 chord 集合一致。
fn normalize_vk(vk: u32) -> u32 {
    match vk {
        v if v == VK_LCONTROL as u32 || v == VK_RCONTROL as u32 => VK_CONTROL as u32,
        v if v == VK_LSHIFT as u32 || v == VK_RSHIFT as u32 => VK_SHIFT as u32,
        v if v == VK_LMENU as u32 || v == VK_RMENU as u32 => VK_MENU as u32,
        v if v == VK_LWIN as u32 || v == VK_RWIN as u32 => VK_LWIN as u32,
        _ => vk,
    }
}

/// 与 build.rs 中的资源 ID 保持一致，直接复用 EXE 内嵌的应用图标。
fn build_tray_icon() -> Result<Icon> {
    let icon = Icon::from_resource(1, None)
        .context("failed to load embedded application icon for tray")?;
    info!("Loaded tray icon from embedded application resource");
    Ok(icon)
}

fn set_tray_menu_text(
    restart_item: &MenuItem,
    exit_item: &MenuItem,
    overlap_item: &CheckMenuItem,
    text: &UiText,
) {
    restart_item.set_text(text.tray_reload_config.replace('&', "&&"));
    exit_item.set_text(text.tray_exit.replace('&', "&&"));
    overlap_item.set_text(text.tray_allow_overlap.replace('&', "&&"));
}

fn ensure_windows_message_queue() {
    unsafe {
        let mut message = MaybeUninit::<MSG>::zeroed();
        let _ = PeekMessageW(
            message.as_mut_ptr(),
            std::ptr::null_mut(),
            0,
            0,
            PM_NOREMOVE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{build_tray_icon, set_tray_menu_text};
    use crate::language::UiText;
    use tray_icon::menu::{CheckMenuItem, ContextMenu, Menu, MenuItem};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMenuItemID, GetMenuState, GetMenuStringW, MF_BYPOSITION, MF_CHECKED,
    };

    #[test]
    fn native_menu_updates_unicode_labels_without_changing_command_ids() {
        let defaults = UiText::default();
        let reload = MenuItem::new(&defaults.tray_reload_config, true, None);
        let exit = MenuItem::new(&defaults.tray_exit, true, None);
        let overlap = CheckMenuItem::new(&defaults.tray_allow_overlap, true, false, None);
        let menu = Menu::with_items(&[&reload, &exit, &overlap]).unwrap();
        let handle = menu.hpopupmenu() as _;
        let command_ids = unsafe {
            [
                GetMenuItemID(handle, 0),
                GetMenuItemID(handle, 1),
                GetMenuItemID(handle, 2),
            ]
        };
        for text in [
            UiText {
                tray_allow_overlap: "Allow Overlapping Audio".into(),
                tray_reload_config: "重新加载配置".into(),
                tray_exit: "退出".into(),
            },
            UiText {
                tray_allow_overlap: "Overlap & Mix".into(),
                tray_reload_config: "Reload & Refresh".into(),
                tray_exit: "Exit && Close".into(),
            },
            defaults,
        ] {
            set_tray_menu_text(&reload, &exit, &overlap, &text);
            overlap.set_checked(true);
            assert!(overlap.is_checked());
            assert_ne!(
                unsafe { GetMenuState(handle, 2, MF_BYPOSITION) } & MF_CHECKED,
                0
            );
            overlap.set_checked(false);
            assert_eq!(
                unsafe { GetMenuState(handle, 2, MF_BYPOSITION) } & MF_CHECKED,
                0
            );
            for (position, expected) in [
                &text.tray_reload_config,
                &text.tray_exit,
                &text.tray_allow_overlap,
            ]
            .iter()
            .enumerate()
            {
                let mut buffer = [0u16; 128];
                let length = unsafe {
                    GetMenuStringW(
                        handle,
                        position as u32,
                        buffer.as_mut_ptr(),
                        buffer.len() as i32,
                        MF_BYPOSITION,
                    )
                };
                assert!(length > 0);
                assert_eq!(
                    String::from_utf16(&buffer[..length as usize]).unwrap(),
                    expected.replace('&', "&&")
                );
                assert_eq!(
                    unsafe { GetMenuItemID(handle, position as i32) },
                    command_ids[position]
                );
            }
        }
    }

    #[test]
    fn tray_icon_loads_from_embedded_application_resource() {
        build_tray_icon().expect("tray icon should load without any external icon file");
    }
}
