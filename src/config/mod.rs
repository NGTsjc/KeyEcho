use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use log::{info, warn};
use serde::Deserialize;

pub const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone)]
pub struct BindingConfig {
    pub id: String,
    pub hotkey: HotkeySpec,
    pub audio_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HotkeySpec {
    pub keys: Vec<KeyCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Control,
    Alt,
    Shift,
    Super,
    Character(char),
    Digit(u8),
    F(u8),
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Minus,
    Equal,
    Comma,
    Period,
    Slash,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    bindings: Vec<RawBindingConfig>,
}

#[derive(Debug, Deserialize)]
struct RawBindingConfig {
    id: String,
    hotkey: String,
    audio_path: String,
}

pub fn default_config_path() -> Result<PathBuf> {
    let executable = std::env::current_exe().context("failed to resolve current executable")?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| anyhow!("executable path has no parent directory"))?;
    Ok(executable_dir.join(CONFIG_FILE_NAME))
}

pub fn load_config(config_path: &Path) -> Result<Vec<BindingConfig>> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file {}", config_path.display()))?;
    let raw_config: RawConfig = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let mut bindings = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut skipped_count = 0usize;
    for (index, raw_binding) in raw_config.bindings.into_iter().enumerate() {
        match validate_binding(raw_binding, config_dir, &mut seen_ids) {
            Ok(binding) => bindings.push(binding),
            Err(error) => {
                skipped_count += 1;
                warn!(
                    "Skipping config binding #{} in {}: {error:#}",
                    index + 1,
                    config_path.display()
                );
            }
        }
    }
    info!(
        "Loaded {} valid binding(s) from {}",
        bindings.len(),
        config_path.display()
    );
    if skipped_count > 0 {
        warn!(
            "Skipped {} invalid binding(s) from {}",
            skipped_count,
            config_path.display()
        );
    }
    Ok(bindings)
}

fn validate_binding(
    raw_binding: RawBindingConfig,
    config_dir: &Path,
    seen_ids: &mut HashSet<String>,
) -> Result<BindingConfig> {
    let id = require_non_empty(raw_binding.id, "id")?;
    if !seen_ids.insert(id.clone()) {
        bail!("duplicate id `{id}`");
    }
    let hotkey_string = require_non_empty(raw_binding.hotkey, "hotkey")?;
    let audio_path_value = require_non_empty(raw_binding.audio_path, "audio_path")?;
    let hotkey = parse_hotkey(&hotkey_string)
        .with_context(|| format!("invalid hotkey `{hotkey_string}` for binding `{id}`"))?;
    let audio_path = resolve_audio_path(config_dir, &audio_path_value)
        .with_context(|| format!("invalid audio_path `{audio_path_value}` for binding `{id}`"))?;
    Ok(BindingConfig {
        id,
        hotkey,
        audio_path,
    })
}

fn require_non_empty(value: String, field_name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{field_name} is empty");
    }
    Ok(trimmed.to_owned())
}

pub fn resolve_audio_path(config_dir: &Path, raw_path: &str) -> Result<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        bail!("audio_path is empty");
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(config_dir.join(path))
}

pub fn parse_hotkey(raw: &str) -> Result<HotkeySpec> {
    let tokens = raw
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        bail!("hotkey string is empty");
    }
    let keys = tokens
        .iter()
        .map(|t| {
            parse_key_code(t).ok_or_else(|| anyhow!("unsupported key token `{t}`"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(HotkeySpec { keys })
}

fn parse_key_code(token: &str) -> Option<KeyCode> {
    let normalized = token.trim().to_ascii_uppercase();
    let bytes = normalized.as_bytes();
    match normalized.as_str() {
        "CTRL" | "CONTROL" => return Some(KeyCode::Control),
        "ALT" => return Some(KeyCode::Alt),
        "SHIFT" => return Some(KeyCode::Shift),
        "SUPER" | "WIN" | "WINDOWS" | "META" | "CMD" | "COMMAND" => return Some(KeyCode::Super),
        _ => {}
    }
    if normalized.len() == 1 {
        if bytes[0].is_ascii_alphabetic() {
            return Some(KeyCode::Character(bytes[0] as char));
        }
        if bytes[0].is_ascii_digit() {
            return Some(KeyCode::Digit(bytes[0] - b'0'));
        }
    }
    if let Some(function_key) = normalized.strip_prefix('F') {
        if let Ok(number) = function_key.parse::<u8>() {
            if (1..=24).contains(&number) {
                return Some(KeyCode::F(number));
            }
        }
    }
    match normalized.as_str() {
        "SPACE" => Some(KeyCode::Space),
        "ENTER" | "RETURN" => Some(KeyCode::Enter),
        "TAB" => Some(KeyCode::Tab),
        "ESC" | "ESCAPE" => Some(KeyCode::Escape),
        "BACKSPACE" => Some(KeyCode::Backspace),
        "DELETE" | "DEL" => Some(KeyCode::Delete),
        "INSERT" | "INS" => Some(KeyCode::Insert),
        "HOME" => Some(KeyCode::Home),
        "END" => Some(KeyCode::End),
        "PAGEUP" | "PGUP" => Some(KeyCode::PageUp),
        "PAGEDOWN" | "PGDN" => Some(KeyCode::PageDown),
        "UP" => Some(KeyCode::Up),
        "DOWN" => Some(KeyCode::Down),
        "LEFT" => Some(KeyCode::Left),
        "RIGHT" => Some(KeyCode::Right),
        "MINUS" | "-" => Some(KeyCode::Minus),
        "EQUAL" | "=" => Some(KeyCode::Equal),
        "COMMA" | "," => Some(KeyCode::Comma),
        "PERIOD" | "DOT" | "." => Some(KeyCode::Period),
        "SLASH" | "/" => Some(KeyCode::Slash),
        "BACKSLASH" | "\\" => Some(KeyCode::Backslash),
        "SEMICOLON" | ";" => Some(KeyCode::Semicolon),
        "QUOTE" | "'" => Some(KeyCode::Quote),
        "BACKQUOTE" | "GRAVE" | "`" => Some(KeyCode::Backquote),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{KeyCode, load_config, parse_hotkey, resolve_audio_path};

    #[test]
    fn parse_hotkey_supports_modifiers_and_keys() {
        let hotkey = parse_hotkey("ctrl + alt + 1").expect("hotkey should parse");
        assert_eq!(
            hotkey.keys,
            vec![KeyCode::Control, KeyCode::Alt, KeyCode::Digit(1)]
        );
    }

    #[test]
    fn parse_hotkey_supports_chord_format() {
        let hotkey = parse_hotkey("CTRL+S+1").expect("hotkey should parse");
        assert_eq!(
            hotkey.keys,
            vec![KeyCode::Control, KeyCode::Character('S'), KeyCode::Digit(1)]
        );
    }

    #[test]
    fn resolve_audio_path_uses_config_directory_for_relative_values() {
        let path = resolve_audio_path(Path::new("D:/apps/keyecho"), "asset/test.wav")
            .expect("path should resolve");
        assert_eq!(path, PathBuf::from("D:/apps/keyecho/asset/test.wav"));
    }

    #[test]
    fn load_config_skips_invalid_entries_and_keeps_valid_ones() {
        let temp_dir = tempdir().expect("temp dir should exist");
        let config_path = temp_dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{
  "bindings": [
    { "id": "valid", "hotkey": "CTRL+SHIFT+K", "audio_path": "asset/test.wav" },
    { "id": "valid", "hotkey": "CTRL+SHIFT+L", "audio_path": "asset/test2.wav" },
    { "id": "bad", "hotkey": "CTRL+UNKNOWN", "audio_path": "asset/test3.wav" }
  ]
}"#,
        )
        .expect("config should write");
        let bindings = load_config(&config_path).expect("config should load");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].id, "valid");
        assert_eq!(
            bindings[0].audio_path,
            temp_dir.path().join("asset").join("test.wav")
        );
    }
}
