//! 51 设置向导：一步式默认、固定掩码、白名单下标、测试单飞。

pub mod input;
pub mod probe;
pub mod view;

pub const SECTION_ORDER: [&str; 4] = ["粘贴 Key", "选择模型", "服务地址", "测试并保存"];
pub const MASK_TOKEN: &str = "***";
pub const PERSIST_FALLBACK_NOTICE: &str = "系统安全存储不可用，本次仅在会话内有效。";

pub use input::{
    EndpointDraft, ModelDraft, PersistTarget, ProbeRequest, SecretInput, SettingsInput,
    SettingsIntent,
};
pub use probe::{
    humanize_probe_failure, next_probe_state, ProbeEvent, ProbeFailureKind, ProbeOutcome,
};
pub use view::{
    apply_defaults, mask_api_key, project_settings_view, resolve_model_name,
    validate_settings_input, EndpointSelectionView, FieldIssue, MaskedKey, ModelSelectionView,
    SettingsDefaults, SettingsFacts, SettingsFormState, SettingsView,
};
