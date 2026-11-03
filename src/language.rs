use std::fs;
use std::path::Path;

use log::warn;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiText {
    pub tray_allow_overlap: String,
    pub tray_reload_config: String,
    pub tray_exit: String,
}

impl Default for UiText {
    fn default() -> Self {
        Self {
            tray_allow_overlap: "Allow Overlapping Audio".to_owned(),
            tray_reload_config: "Reload Config".to_owned(),
            tray_exit: "Exit".to_owned(),
        }
    }
}

/// Relative language paths use the selected configuration file's directory.
pub fn load_ui_text(config_path: &Path, language_file: Option<&str>) -> UiText {
    let Some(file) = language_file.map(str::trim).filter(|file| !file.is_empty()) else {
        return UiText::default();
    };
    let path = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file);
    let mut text = UiText::default();
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            warn!(
                "Failed to read language file {}: {error}; using English defaults",
                path.display()
            );
            return text;
        }
    };
    let overrides =
        match serde_json::from_str::<Map<String, Value>>(content.trim_start_matches('\u{feff}')) {
            Ok(overrides) => overrides,
            Err(error) => {
                warn!(
                    "Failed to parse language file {}: {error}; using English defaults",
                    path.display()
                );
                return text;
            }
        };
    apply_override(
        &overrides,
        "tray_reload_config",
        &mut text.tray_reload_config,
        &path,
    );
    apply_override(&overrides, "tray_exit", &mut text.tray_exit, &path);
    apply_override(
        &overrides,
        "tray_allow_overlap",
        &mut text.tray_allow_overlap,
        &path,
    );
    text
}

fn apply_override(overrides: &Map<String, Value>, key: &str, target: &mut String, path: &Path) {
    let Some(value) = overrides.get(key) else {
        return;
    };
    match value.as_str() {
        Some(value) if !value.trim().is_empty() && !value.chars().any(char::is_control) => {
            *target = value.trim().to_owned();
        }
        _ => warn!(
            "Invalid language field `{key}` in {}: expected a nonempty string without control characters; using English default",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{UiText, load_ui_text};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_file_uses_english_defaults() {
        let dir = tempdir().unwrap();
        let text = load_ui_text(&dir.path().join("config.json"), Some("language.json"));
        assert_eq!(text.tray_reload_config, "Reload Config");
        assert_eq!(text.tray_exit, "Exit");
    }

    #[test]
    fn complete_and_partial_overrides_are_loaded_beside_config() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.json");
        let path = dir.path().join("language.json");
        fs::write(
            &path,
            r#"{"tray_reload_config":"重新加载配置","tray_exit":"退出"}"#,
        )
        .unwrap();
        assert_eq!(
            load_ui_text(&config, Some("language.json")),
            UiText {
                tray_allow_overlap: "Allow Overlapping Audio".into(),
                tray_reload_config: "重新加载配置".into(),
                tray_exit: "退出".into(),
            }
        );
        fs::write(&path, r#"{"tray_exit":"  Quit  ","future_key":42}"#).unwrap();
        assert_eq!(
            load_ui_text(&config, Some("language.json")),
            UiText {
                tray_exit: "Quit".into(),
                ..UiText::default()
            }
        );
    }

    #[test]
    fn explicit_selection_supports_relative_absolute_bom_and_shipped_example() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.json");
        let example = include_str!("../asset/locale.example.json");
        fs::write(dir.path().join("language.json"), example).unwrap();
        for selection in [None, Some(""), Some("  ")] {
            assert_eq!(load_ui_text(&config, selection), UiText::default());
        }
        let locales = dir.path().join("locales");
        fs::create_dir(&locales).unwrap();
        let path = locales.join("zh-CN.json");
        fs::write(&path, format!("\u{feff}{example}")).unwrap();
        let expected = UiText {
            tray_allow_overlap: "允许音频叠加".into(),
            tray_reload_config: "重新加载配置".into(),
            tray_exit: "退出".into(),
        };
        for selection in ["locales/zh-CN.json", path.to_str().unwrap()] {
            assert_eq!(load_ui_text(&config, Some(selection)), expected);
        }
    }

    #[test]
    fn invalid_fields_do_not_discard_other_valid_overrides() {
        let dir = tempdir().unwrap();
        for invalid in [
            serde_json::json!(""),
            serde_json::json!("  "),
            serde_json::json!(null),
            serde_json::json!(42),
            serde_json::json!(false),
            serde_json::json!([]),
            serde_json::json!({}),
            serde_json::json!("Exit\0hidden"),
            serde_json::json!("Exit\tCtrl+Q"),
            serde_json::json!("Exit\nnow"),
            serde_json::json!("\tQuit"),
        ] {
            fs::write(
                dir.path().join("language.json"),
                format!(r#"{{"tray_reload_config":{invalid},"tray_allow_overlap":{invalid},"tray_exit":"退出"}}"#),
            )
            .unwrap();
            assert_eq!(
                load_ui_text(&dir.path().join("config.json"), Some("language.json")),
                UiText {
                    tray_exit: "退出".into(),
                    ..UiText::default()
                },
                "invalid field: {invalid}"
            );
        }
    }

    #[test]
    fn invalid_files_fall_back_to_english() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("language.json");
        for content in ["{broken", "[]", "null", "42", r#""text""#, "{}"] {
            fs::write(&path, content).unwrap();
            assert_eq!(
                load_ui_text(&dir.path().join("config.json"), Some("language.json")),
                UiText::default()
            );
        }
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert_eq!(
            load_ui_text(&dir.path().join("config.json"), Some("language.json")),
            UiText::default()
        );
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(
            load_ui_text(&dir.path().join("config.json"), Some("language.json")),
            UiText::default()
        );
    }
}
