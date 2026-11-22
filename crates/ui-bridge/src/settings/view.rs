//! 设置视图模型：掩码、模型/端点显示名、内联错误与默认值。

use serde::Serialize;

use super::input::{EndpointDraft, ModelDraft, SecretInput, SettingsInput};

pub const MASK_TOKEN: &str = "***";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsFacts {
    pub has_stored_key: bool,
    pub model_count: usize,
    pub endpoint_count: usize,
    pub now_ms: u64,
    pub cooldown_until_ms: Option<u64>,
    pub probe_in_flight: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsDefaults {
    pub model_index: usize,
    pub endpoint_index: usize,
    pub timeout_ms: u64,
    pub auto_save_on_verified: bool,
}

impl SettingsDefaults {
    pub const fn single_step() -> Self {
        Self {
            model_index: 0,
            endpoint_index: 0,
            timeout_ms: 6_000,
            auto_save_on_verified: true,
        }
    }
}

impl Default for SettingsDefaults {
    fn default() -> Self {
        Self::single_step()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MaskedKey {
    Masked,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ModelSelectionView {
    Preset { display_name: String },
    Custom { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum EndpointSelectionView {
    Option { display_name: String },
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FieldIssue {
    ApiKeyMissing,
    ApiKeyTooShort,
    ApiKeyHasWhitespace,
    ModelMissing,
    ModelTooLong,
    EndpointUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SettingsFormState {
    NotConfigured,
    ConfiguredUnverified,
    Testing,
    Verified,
    Failed,
    Cooling { remaining_ms: u64 },
}

#[derive(Clone, PartialEq, Serialize)]
pub struct SettingsView {
    pub form_state: SettingsFormState,
    pub has_key: bool,
    pub key_display: MaskedKey,
    pub model_choice: ModelSelectionView,
    pub endpoint_choice: EndpointSelectionView,
    pub inline_errors: Vec<FieldIssue>,
    pub can_test: bool,
    pub can_save: bool,
}

impl core::fmt::Debug for SettingsView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SettingsView")
            .field("form_state", &self.form_state)
            .field("has_key", &self.has_key)
            .field("key_display", &self.key_display)
            .field("has_errors", &!self.inline_errors.is_empty())
            .field("can_test", &self.can_test)
            .field("can_save", &self.can_save)
            .finish()
    }
}

pub fn mask_api_key(input: &SecretInput) -> MaskedKey {
    if input.is_provided() {
        MaskedKey::Masked
    } else {
        MaskedKey::None
    }
}

pub fn resolve_model_name(draft: &ModelDraft, presets: &[String]) -> ModelSelectionView {
    match draft {
        ModelDraft::PresetIndex(index) => match presets.get(*index) {
            Some(name) => ModelSelectionView::Preset {
                display_name: name.clone(),
            },
            None => ModelSelectionView::Custom {
                name: String::new(),
            },
        },
        ModelDraft::CustomName(name) => ModelSelectionView::Custom { name: name.clone() },
        ModelDraft::None => ModelSelectionView::Custom {
            name: String::new(),
        },
    }
}

pub fn apply_defaults(facts: &SettingsFacts, defaults: &SettingsDefaults) -> SettingsInput {
    SettingsInput {
        api_key_draft: if facts.has_stored_key {
            SecretInput::stored()
        } else {
            SecretInput::empty()
        },
        model_draft: if facts.model_count > defaults.model_index {
            ModelDraft::PresetIndex(defaults.model_index)
        } else {
            ModelDraft::None
        },
        endpoint_draft: if facts.endpoint_count > defaults.endpoint_index {
            EndpointDraft::Index(defaults.endpoint_index)
        } else {
            EndpointDraft::None
        },
        submitted: false,
        clear_confirmed: false,
    }
}

pub fn validate_settings_input(
    facts: &SettingsFacts,
    input: &SettingsInput,
    presets: &[String],
    _endpoints: &[String],
) -> Vec<FieldIssue> {
    if !input.submitted {
        return Vec::new();
    }
    let mut issues = Vec::new();
    if let Some(issue) = input.api_key_draft.draft_issue() {
        issues.push(issue);
    }
    match &input.model_draft {
        ModelDraft::None => issues.push(FieldIssue::ModelMissing),
        ModelDraft::CustomName(name) if name.is_empty() || name.contains(' ') => {
            issues.push(FieldIssue::ModelMissing)
        }
        ModelDraft::CustomName(name) if name.chars().count() > 128 => {
            issues.push(FieldIssue::ModelTooLong)
        }
        ModelDraft::PresetIndex(_) if presets.is_empty() => issues.push(FieldIssue::ModelMissing),
        _ => {}
    }
    if facts.endpoint_count == 0 {
        issues.push(FieldIssue::EndpointUnavailable);
    }
    issues.sort_by_key(|issue| *issue as u8);
    issues.dedup();
    issues
}

pub fn project_settings_view(
    facts: &SettingsFacts,
    input: &SettingsInput,
    state: SettingsFormState,
    presets: &[String],
    endpoints: &[String],
) -> SettingsView {
    let has_key = input.api_key_draft.is_provided();
    let model_choice = resolve_model_name(&input.model_draft, presets);
    let endpoint_choice = match input.endpoint_draft {
        EndpointDraft::Index(index) => endpoints
            .get(index)
            .map(|name| EndpointSelectionView::Option {
                display_name: name.clone(),
            })
            .unwrap_or(EndpointSelectionView::Unavailable),
        EndpointDraft::None => EndpointSelectionView::Unavailable,
    };
    let form_state = if !has_key {
        SettingsFormState::NotConfigured
    } else if facts.probe_in_flight {
        SettingsFormState::Testing
    } else if let Some(until) = facts.cooldown_until_ms {
        if until > facts.now_ms {
            SettingsFormState::Cooling {
                remaining_ms: until - facts.now_ms,
            }
        } else {
            state
        }
    } else {
        state
    };
    let model_valid =
        !matches!(model_choice, ModelSelectionView::Custom { ref name } if name.is_empty());
    let endpoint_ok = !matches!(endpoint_choice, EndpointSelectionView::Unavailable);
    let busy = matches!(
        form_state,
        SettingsFormState::Testing | SettingsFormState::Cooling { .. }
    );
    SettingsView {
        form_state,
        has_key,
        key_display: mask_api_key(&input.api_key_draft),
        model_choice,
        endpoint_choice,
        inline_errors: validate_settings_input(facts, input, presets, endpoints),
        can_test: has_key && model_valid && endpoint_ok && !busy,
        can_save: has_key && model_valid && endpoint_ok,
    }
}
