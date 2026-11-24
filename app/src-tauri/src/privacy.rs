//! 开发期隐私态注入：只用于真机验收"投屏 / 紧急遮蔽"（`open-items.md §8.3` 第 8 项）。
//!
//! 产品面目前没有设置这两个状态的入口，真机因此无法自然触发遮蔽。此模块提供**只读、默认关闭**
//! 的开发期注入，让验收可以在不改动 IPC 命令面与门控判定的前提下走通：
//!
//! - `PRIVACY_SCREEN_SHARE=1` → 视为正在投屏
//! - `PRIVACY_EMERGENCY_MASKED=1` → 视为紧急遮蔽中
//!
//! 只认严格的 `"1"`；空值、`true`、` 1` 等一律视为关闭，避免误开导致真机验收结论失真。
//! 两个开关都不落库、不进日志、不投影给前端。

/// 隐私态快照；默认全关，与加入本模块前的行为完全一致。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrivacyMode {
    pub screen_share: bool,
    pub emergency_masked: bool,
}

impl PrivacyMode {
    pub fn from_env() -> Self {
        Self {
            screen_share: flag("PRIVACY_SCREEN_SHARE"),
            emergency_masked: flag("PRIVACY_EMERGENCY_MASKED"),
        }
    }
}

fn flag(key: &str) -> bool {
    matches!(std::env::var(key).as_deref(), Ok("1"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_flags_default_to_off() {
        assert!(!flag("PRIVACY_MISSING_KEY_UNDER_TEST"));
    }

    #[test]
    fn default_mode_is_fully_visible() {
        let mode = PrivacyMode::default();
        assert!(!mode.screen_share);
        assert!(!mode.emergency_masked);
    }
}
