//! 设置草稿与意图：凭据只在内存，Debug 恒 `***`。

use super::view::FieldIssue;

#[derive(Clone, PartialEq)]
pub struct SecretInput {
    draft: Option<String>,
    stored: bool,
}

impl SecretInput {
    pub fn empty() -> Self {
        Self {
            draft: None,
            stored: false,
        }
    }

    pub fn stored() -> Self {
        Self {
            draft: None,
            stored: true,
        }
    }

    pub fn from_draft(raw: &str) -> Self {
        let trimmed = raw.trim();
        Self {
            draft: if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            },
            stored: false,
        }
    }

    pub fn is_provided(&self) -> bool {
        self.stored || self.draft.is_some()
    }

    pub fn is_stored(&self) -> bool {
        self.stored
    }

    pub fn draft_issue(&self) -> Option<FieldIssue> {
        if self.stored {
            return None;
        }
        let Some(raw) = &self.draft else {
            return Some(FieldIssue::ApiKeyMissing);
        };
        if raw.chars().count() < 16 {
            return Some(FieldIssue::ApiKeyTooShort);
        }
        if raw.chars().any(char::is_whitespace) {
            return Some(FieldIssue::ApiKeyHasWhitespace);
        }
        None
    }
}

impl core::fmt::Debug for SecretInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("***")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelDraft {
    PresetIndex(usize),
    CustomName(String),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointDraft {
    Index(usize),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistTarget {
    Memory,
    OsSecureStore,
}

#[derive(Clone)]
pub struct ProbeRequest {
    pub credential: SecretInput,
    pub model: ModelDraft,
    pub endpoint_index: Option<usize>,
    pub timeout_ms: u64,
}

impl core::fmt::Debug for ProbeRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProbeRequest")
            .field("has_credential", &self.credential.is_provided())
            .field("model", &self.model)
            .field("endpoint_index", &self.endpoint_index)
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

#[derive(Clone)]
pub struct SettingsInput {
    pub api_key_draft: SecretInput,
    pub model_draft: ModelDraft,
    pub endpoint_draft: EndpointDraft,
    pub submitted: bool,
    pub clear_confirmed: bool,
}

impl core::fmt::Debug for SettingsInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SettingsInput")
            .field("has_key", &self.api_key_draft.is_provided())
            .field("model", &self.model_draft)
            .field("endpoint", &self.endpoint_draft)
            .field("submitted", &self.submitted)
            .field("clear_confirmed", &self.clear_confirmed)
            .finish()
    }
}

#[derive(Clone)]
pub enum SettingsIntent {
    Persist { target: PersistTarget },
    Clear,
    Probe { request: ProbeRequest },
}

impl core::fmt::Debug for SettingsIntent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Persist { target } => f.debug_tuple("Persist").field(target).finish(),
            Self::Clear => f.write_str("Clear"),
            Self::Probe { request } => f.debug_tuple("Probe").field(request).finish(),
        }
    }
}

impl SettingsInput {
    pub fn persist_intent(&self, target: PersistTarget) -> Option<SettingsIntent> {
        if self.api_key_draft.is_provided() {
            Some(SettingsIntent::Persist { target })
        } else {
            None
        }
    }

    pub fn clear_intent(&self) -> Option<SettingsIntent> {
        if self.clear_confirmed && self.api_key_draft.is_provided() {
            Some(SettingsIntent::Clear)
        } else {
            None
        }
    }
}
