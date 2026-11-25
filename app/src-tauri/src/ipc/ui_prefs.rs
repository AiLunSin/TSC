//! 界面偏好：状态灯与卡片的文字显示方式（悬停显示 / 常显）。
//!
//! 边界：只有两个固定取值，不认识的值一律回 `hover`；文件损坏或版本不符同样回默认。
//! 偏好坏了不该挡住开会，所以读失败不报错、写失败只回一条可重试的短句。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use meeting_copilot_application::settings::PortError;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ipc::dto::{IpcErrorDto, UiPrefsDto, UiPrefsInputDto};

/// 配置文件：界面偏好。与 `settings.json`（恰 4 键）、`assist.json` 分开。
const CONFIG_FILE: &str = "ui.json";
/// 悬停才显示文字（默认，窄栏）。
const LABEL_HOVER: &str = "hover";
/// 一直显示文字（宽栏）。
const LABEL_ALWAYS: &str = "always";

/// 落盘的界面偏好：版本不符或损坏直接回默认。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UiFileV1 {
    schema_version: u32,
    label_mode: String,
}

impl Default for UiFileV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            label_mode: LABEL_HOVER.to_owned(),
        }
    }
}

/// 只接受两个已知取值；其余（含旧版本写入的怪值）当作默认。
fn sanitize_label_mode(raw: &str) -> &'static str {
    if raw == LABEL_ALWAYS {
        LABEL_ALWAYS
    } else {
        LABEL_HOVER
    }
}

/// 界面偏好存储：临时文件 + 同目录替换。
pub struct UiPrefsStore {
    dir: PathBuf,
}

impl UiPrefsStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn main_path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }

    fn load(&self) -> UiFileV1 {
        let Ok(text) = fs::read_to_string(self.main_path()) else {
            return UiFileV1::default();
        };
        match serde_json::from_str::<UiFileV1>(&text) {
            Ok(file) if file.schema_version == 1 => file,
            _ => UiFileV1::default(),
        }
    }

    fn save(&self, file: &UiFileV1) -> Result<(), PortError> {
        fs::create_dir_all(&self.dir).map_err(|_| PortError::Unavailable)?;
        let json = serde_json::to_vec_pretty(file).map_err(|_| PortError::Corrupted)?;
        let tmp = self.dir.join("ui.json.tmp");
        fs::write(&tmp, json).map_err(|_| PortError::Unavailable)?;
        fs::rename(&tmp, self.main_path()).map_err(|_| PortError::Unavailable)
    }
}

/// 界面偏好运行态：只有显示方式一项。
pub struct UiPrefsState {
    store: UiPrefsStore,
}

impl UiPrefsState {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            store: UiPrefsStore::new(dir),
        }
    }

    pub fn prefs(&self) -> UiPrefsDto {
        UiPrefsDto {
            label_mode: sanitize_label_mode(&self.store.load().label_mode).to_owned(),
        }
    }

    pub fn save_prefs(&self, input: &UiPrefsInputDto) -> Result<UiPrefsDto, IpcErrorDto> {
        let file = UiFileV1 {
            schema_version: 1,
            label_mode: sanitize_label_mode(&input.label_mode).to_owned(),
        };
        self.store
            .save(&file)
            .map_err(|_| IpcErrorDto::new("ui_prefs_unwritable", "界面偏好保存失败。", true))?;
        Ok(self.prefs())
    }
}

#[tauri::command]
pub fn ui_prefs(state: State<'_, Arc<UiPrefsState>>) -> UiPrefsDto {
    state.prefs()
}

#[tauri::command]
pub fn ui_prefs_save(
    state: State<'_, Arc<UiPrefsState>>,
    input: UiPrefsInputDto,
) -> Result<UiPrefsDto, IpcErrorDto> {
    state.save_prefs(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .join("ui-prefs-tests")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn missing_file_falls_back_to_hover() {
        let state = UiPrefsState::new(temp_dir("missing"));
        assert_eq!(state.prefs().label_mode, LABEL_HOVER);
    }

    #[test]
    fn saves_and_reloads_always() {
        let dir = temp_dir("roundtrip");
        let state = UiPrefsState::new(dir.clone());
        let saved = state
            .save_prefs(&UiPrefsInputDto {
                label_mode: LABEL_ALWAYS.to_owned(),
            })
            .expect("save");
        assert_eq!(saved.label_mode, LABEL_ALWAYS);
        assert_eq!(UiPrefsState::new(dir).prefs().label_mode, LABEL_ALWAYS);
    }

    #[test]
    fn unknown_value_is_stored_as_hover() {
        let dir = temp_dir("unknown");
        let state = UiPrefsState::new(dir.clone());
        let saved = state
            .save_prefs(&UiPrefsInputDto {
                label_mode: "sideways".to_owned(),
            })
            .expect("save");
        assert_eq!(saved.label_mode, LABEL_HOVER);
        let text = fs::read_to_string(dir.join(CONFIG_FILE)).expect("file");
        assert!(!text.contains("sideways"));
    }

    #[test]
    fn corrupted_file_falls_back_to_hover() {
        let dir = temp_dir("corrupted");
        fs::write(dir.join(CONFIG_FILE), b"{ not json").expect("write");
        assert_eq!(UiPrefsState::new(dir).prefs().label_mode, LABEL_HOVER);
    }
}
