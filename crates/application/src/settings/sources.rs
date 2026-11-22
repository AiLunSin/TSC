//! 只读事实源：设置事实、卡片快照、视图事实（3 + 3 结构）。

use meeting_copilot_domain::model::gate_keyword::KeywordCardDraft;

use crate::cards::ComposedCard;

/// 设置事实快照：恰 6 项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactsSnapshot {
    /// 是否已有可用 Key（只判存在，不取内容）。
    pub has_stored_key: bool,
    /// 目录清单中的模型数量。
    pub model_count: usize,
    /// 目录清单中的端点数量。
    pub endpoint_count: usize,
    /// 命令内取一次的毫秒时点。
    pub now_ms: u64,
    /// 冷却截止绝对毫秒镜像；与端口相对值 `remaining_ms` 区分。
    pub cooldown_until_ms: Option<u64>,
    /// 探针是否在途。
    pub probe_in_flight: bool,
}

/// 视图事实快照：恰 6 项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFactsSnapshot {
    /// 当前会话是否匹配。
    pub session_matches: bool,
    /// 授权是否已撤回。
    pub authorization_revoked: bool,
    /// 引用是否过期。
    pub reference_stale: bool,
    /// 是否处于投屏保护。
    pub screen_share: bool,
    /// 是否紧急遮蔽。
    pub emergency_masked: bool,
    /// 可见卡片条数。
    pub visible_card_count: u32,
}

/// 卡片快照条目：2 态，读取时不得合成。
#[derive(Debug, Clone, PartialEq)]
pub enum CardSnapshotEntry {
    /// 13 字段卡片草案，交 51 投影。
    Draft(KeywordCardDraft),
    /// 11 项预览，恒不可点。
    Preview(ComposedCard),
}

/// 设置事实源：只读产出 6 项事实。
pub trait SettingsFactsSource: Send + Sync {
    /// 读取事实快照。
    fn facts(&self) -> FactsSnapshot;
}

/// 卡片快照源：只读注册表/注入。
pub trait CardSnapshotSource: Send + Sync {
    /// 读取卡片条目。
    fn cards(&self) -> Vec<CardSnapshotEntry>;
}

/// 视图事实源：只消费注入事实。
pub trait ViewFactsSource: Send + Sync {
    /// 读取视图事实。
    fn view_facts(&self) -> ViewFactsSnapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = include_str!("sources.rs");

    fn field_count(name: &str) -> usize {
        let needle = format!("pub struct {name} {{");
        let start = SOURCE.find(&needle).expect("struct present");
        let mut inside = false;
        let mut fields = 0usize;
        for line in SOURCE[start..].lines() {
            if !inside {
                inside = true;
                continue;
            }
            if line.trim() == "}" {
                break;
            }
            if line.starts_with("    pub ") {
                fields += 1;
            }
        }
        fields
    }

    #[test]
    fn sources_declare_three_ports() {
        for name in [
            "pub trait SettingsFactsSource",
            "pub trait CardSnapshotSource",
            "pub trait ViewFactsSource",
        ] {
            assert!(SOURCE.contains(name), "missing {name}");
        }
        assert_eq!(field_count("FactsSnapshot"), 6);
        assert_eq!(field_count("ViewFactsSnapshot"), 6);
        assert!(SOURCE.contains("Draft(KeywordCardDraft)"));
        assert!(SOURCE.contains("Preview(ComposedCard)"));
    }

    #[test]
    fn empty_card_source_is_legal() {
        struct EmptyCards;

        impl CardSnapshotSource for EmptyCards {
            fn cards(&self) -> Vec<CardSnapshotEntry> {
                Vec::new()
            }
        }

        assert!(EmptyCards.cards().is_empty());
    }
}
