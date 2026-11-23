//! 非敏感配置：恰 4 键、原子写、损坏或版本不符回默认并留 `.bak`。

use std::fs;
use std::path::PathBuf;

use meeting_copilot_application::settings::{PortError, SettingsConfig, SettingsConfigStore};
use serde::{Deserialize, Serialize};

/// 配置文件的 4 键形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsFileV1 {
    pub schema_version: u32,
    pub model_index: usize,
    pub endpoint_index: usize,
    pub timeout_ms: u64,
}

impl SettingsFileV1 {
    fn from_config(config: &SettingsConfig) -> Self {
        Self {
            schema_version: config.schema_version,
            model_index: config.model_index,
            endpoint_index: config.endpoint_index,
            timeout_ms: config.timeout_ms,
        }
    }

    fn into_config(self) -> SettingsConfig {
        SettingsConfig {
            schema_version: self.schema_version,
            model_index: self.model_index,
            endpoint_index: self.endpoint_index,
            timeout_ms: self.timeout_ms,
        }
    }
}

impl Default for SettingsFileV1 {
    fn default() -> Self {
        Self::from_config(&SettingsConfig::default())
    }
}

/// 文件配置存储：临时文件 + 同目录替换。
pub struct FileSettingsConfigStore {
    dir: PathBuf,
}

impl FileSettingsConfigStore {
    /// 以配置目录构造；文件名固定为 `settings.json`。
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn main_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    fn tmp_path(&self) -> PathBuf {
        self.dir.join("settings.json.tmp")
    }

    fn bak_path(&self) -> PathBuf {
        self.dir.join("settings.json.bak")
    }

    fn write_atomic(&self, file: &SettingsFileV1) -> Result<(), PortError> {
        let json = serde_json::to_vec_pretty(file).map_err(|_| PortError::Corrupted)?;
        let tmp = self.tmp_path();
        fs::write(&tmp, json).map_err(|_| PortError::Unavailable)?;
        std::fs::rename(&tmp, self.main_path()).map_err(|_| PortError::Unavailable)
    }

    fn backup_default(&self) {
        let _ = serde_json::to_vec_pretty(&SettingsFileV1::default())
            .map(|json| fs::write(self.bak_path(), json));
    }
}

impl SettingsConfigStore for FileSettingsConfigStore {
    fn load(&self) -> Result<SettingsConfig, PortError> {
        let text = match fs::read_to_string(self.main_path()) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SettingsConfig::default());
            }
            Err(_) => return Err(PortError::Unavailable),
        };

        let parsed = serde_json::from_str::<SettingsFileV1>(&text);
        let accepted = parsed.as_ref().is_ok_and(|file| file.schema_version == 1);
        if !accepted {
            self.backup_default();
            return Ok(SettingsConfig::default());
        }
        Ok(parsed.map(SettingsFileV1::into_config).unwrap_or_default())
    }

    fn save(&self, config: &SettingsConfig) -> Result<(), PortError> {
        fs::create_dir_all(&self.dir).map_err(|_| PortError::Unavailable)?;
        self.write_atomic(&SettingsFileV1::from_config(config))
    }

    fn reset(&self) -> Result<(), PortError> {
        match fs::remove_file(self.main_path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(PortError::Unavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .join("ui-store-tests")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn ipc_settings_file_has_only_four_keys() {
        let dir = temp_dir("four-keys");
        let store = FileSettingsConfigStore::new(&dir);
        store
            .save(&SettingsConfig {
                schema_version: 1,
                model_index: 1,
                endpoint_index: 0,
                timeout_ms: 4_500,
            })
            .expect("save");

        let text = fs::read_to_string(dir.join("settings.json")).expect("read");
        let value: serde_json::Value = serde_json::from_str(&text).expect("json");
        let object = value.as_object().expect("object");
        assert_eq!(object.len(), 4);
        for key in ["schemaVersion", "modelIndex", "endpointIndex", "timeoutMs"] {
            assert!(object.contains_key(key), "missing {key}");
        }
    }

    #[test]
    fn ipc_settings_corruption_falls_back_to_defaults() {
        let dir = temp_dir("corrupted");
        let store = FileSettingsConfigStore::new(&dir);
        fs::write(dir.join("settings.json"), b"{not-json").expect("write corrupt");
        assert_eq!(
            store.load().expect("load corrupt"),
            SettingsConfig::default()
        );
        assert!(dir.join("settings.json.bak").exists());

        let dir = temp_dir("version-mismatch");
        let store = FileSettingsConfigStore::new(&dir);
        let v2 = br#"{"schemaVersion":2,"modelIndex":3,"endpointIndex":2,"timeoutMs":9000}"#;
        fs::write(dir.join("settings.json"), v2).expect("write v2");
        assert_eq!(store.load().expect("load v2"), SettingsConfig::default());
        assert!(dir.join("settings.json.bak").exists());
    }

    #[test]
    fn ipc_settings_missing_file_is_default() {
        let dir = temp_dir("missing");
        let store = FileSettingsConfigStore::new(&dir);
        assert_eq!(
            store.load().expect("load missing"),
            SettingsConfig::default()
        );
    }

    #[test]
    fn reset_removes_file_and_load_is_default() {
        let dir = temp_dir("reset-removes");
        let store = FileSettingsConfigStore::new(&dir);
        store
            .save(&SettingsConfig {
                schema_version: 1,
                model_index: 1,
                endpoint_index: 0,
                timeout_ms: 4_500,
            })
            .expect("save");
        assert!(dir.join("settings.json").exists());

        store.reset().expect("reset");
        assert!(!dir.join("settings.json").exists());
        assert_eq!(store.load().expect("load"), SettingsConfig::default());
        store.reset().expect("reset twice");
    }
}
