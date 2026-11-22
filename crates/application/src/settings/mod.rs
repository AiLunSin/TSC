//! 51 设置向导的应用层契约：3 端口 + 3 只读事实源。

pub mod ports;
pub mod sources;

pub use ports::{
    PortError, ProbeAttempt, ProbeCall, ProbeExecutor, ProbeFailure, SecretBytes, SecureStore,
    SettingsConfig, SettingsConfigStore, SETTINGS_DEFAULT_SCHEMA_VERSION,
    SETTINGS_DEFAULT_TIMEOUT_MS, SETTINGS_TIMEOUT_MAX_MS, SETTINGS_TIMEOUT_MIN_MS,
};
pub use sources::{
    CardSnapshotEntry, CardSnapshotSource, FactsSnapshot, SettingsFactsSource, ViewFactsSnapshot,
    ViewFactsSource,
};
