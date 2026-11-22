//! 规则预筛与信号分类（不调用网关）。

use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::insight_kind::{classify_kind, KindClassification};

pub const STATEMENT_SIGNALS: &[&str] = &["确认", "上线", "决定", "是", "为", "将", "已经"];
pub const ACTION_SIGNALS: &[&str] = &["我来", "跟进", "负责", "下周", "安排", "待办"];
pub const QUESTION_SIGNALS: &[&str] = &["？", "?", "吗", "是否", "如何", "为什么", "能否"];

fn any_hit(text: &str, signals: &[&str]) -> bool {
    signals.iter().any(|signal| text.contains(signal))
}

/// 三信号任一命中即进入抽取；全不命中短路。
pub fn prefilter_hit(text: &str) -> bool {
    any_hit(text, STATEMENT_SIGNALS)
        || any_hit(text, ACTION_SIGNALS)
        || any_hit(text, QUESTION_SIGNALS)
}

/// 兜底时按预筛信号定 kind；未知一律 `KeyInfo` 并标记兜底。
pub fn classify_signals(text: &str) -> KindClassification {
    if any_hit(text, QUESTION_SIGNALS) {
        return KindClassification {
            kind: InsightKind::Question,
            fallback: false,
        };
    }
    if any_hit(text, ACTION_SIGNALS) {
        return KindClassification {
            kind: InsightKind::Todo,
            fallback: false,
        };
    }
    if any_hit(text, STATEMENT_SIGNALS) {
        return classify_kind(Some("decision"));
    }
    classify_kind(None)
}

/// 抽取决策与编排统一从本模块暴露（30.1 合名）。
pub use super::insight_gateway::{decide_extraction, ExtractDecision};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefilter_detects_statement_action_question() {
        assert!(prefilter_hit("确认下周灰度上线"));
        assert!(prefilter_hit("我来负责回滚"));
        assert!(prefilter_hit("是否已经上线？"));
    }

    #[test]
    fn prefilter_miss_for_plain_filler() {
        assert!(!prefilter_hit("嗯，好的"));
        assert!(!prefilter_hit("……"));
    }

    #[test]
    fn classify_signals_maps_question_action_statement() {
        assert_eq!(classify_signals("是否上线？").kind, InsightKind::Question);
        assert_eq!(classify_signals("我来跟进").kind, InsightKind::Todo);
        assert_eq!(classify_signals("确认已经上线").kind, InsightKind::Decision);
        assert!(classify_signals("嗯好的").fallback);
    }
}
