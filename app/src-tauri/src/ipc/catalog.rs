//! 目录清单：注入预设 + 既有白名单首项端点；只暴露显示名。

const DEFAULT_ENDPOINT_LABEL: &str = "默认服务";

/// 端点目标：只保留显示名，不携带 host/地址。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointTarget {
    pub display_name: String,
}

impl EndpointTarget {
    /// 由固定显示名构造。
    pub fn new(display_name: String) -> Self {
        Self { display_name }
    }
}

/// 目录清单：模型预设与端点。
#[derive(Clone, PartialEq, Eq)]
pub struct SettingsCatalog {
    pub presets: Vec<String>,
    pub endpoints: Vec<EndpointTarget>,
}

impl SettingsCatalog {
    /// 由注入预设生成；端点固定为既有白名单首项。
    pub fn from_injected(presets: Vec<String>) -> Self {
        Self {
            presets,
            endpoints: vec![EndpointTarget::new(DEFAULT_ENDPOINT_LABEL.to_owned())],
        }
    }

    /// 模型数量。
    pub fn model_count(&self) -> usize {
        self.presets.len()
    }

    /// 端点数量。
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
}

impl core::fmt::Debug for SettingsCatalog {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SettingsCatalog")
            .field("model_count", &self.model_count())
            .field("endpoint_count", &self.endpoint_count())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use meeting_copilot_infrastructure::llm::live::DEEPSEEK_HOSTS;

    use super::*;

    #[test]
    fn catalog_uses_injected_presets_and_fixed_label() {
        let catalog = SettingsCatalog::from_injected(vec!["preset-a".to_owned()]);
        assert_eq!(catalog.model_count(), 1);
        assert_eq!(catalog.endpoint_count(), 1);
        assert_eq!(catalog.endpoints[0].display_name, DEFAULT_ENDPOINT_LABEL);
        assert!(!format!("{:?}", catalog.endpoints[0]).contains(DEEPSEEK_HOSTS[0]));
    }
}
