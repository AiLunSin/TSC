//! 零源模式（SPEC 8.2-4、8.5）。
//!
//! 启用检索源为 0 时，按需补全与自动检索直接返回 `NoSource`：
//! 不查缓存、不发连接、不写样本；要点与纪要路径不经过本模块。

use crate::proactive::on_demand::OnDemandDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlyPointsMode {
    /// 没有可用检索源，只能使用本场要点。
    Active,
    Inactive,
}

impl OnlyPointsMode {
    pub fn from_usable_count(usable_sources: usize) -> Self {
        if usable_sources == 0 {
            Self::Active
        } else {
            Self::Inactive
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    pub const fn decision(self) -> OnDemandDecision {
        OnDemandDecision::NoSource
    }

    /// 零源模式绝不发起网络请求。
    pub const fn should_attempt_network(self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sources_switch_to_points_only() {
        let mode = OnlyPointsMode::from_usable_count(0);
        assert!(mode.is_active());
        assert_eq!(mode.decision(), OnDemandDecision::NoSource);
        assert!(!mode.should_attempt_network());
    }

    #[test]
    fn any_usable_source_disables_points_only() {
        let mode = OnlyPointsMode::from_usable_count(1);
        assert!(!mode.is_active());
    }
}
