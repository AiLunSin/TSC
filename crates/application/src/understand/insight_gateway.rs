//! 抽取网关的应用层适配：能力前置、结果转草稿、来源校验。
//!
//! 真正的网络调用由 `StructuredLlmGateway` 承担；本模块保持可离线判定。

use meeting_copilot_domain::model::insight::{InsightDraft, InsightFields, InsightSources};
use meeting_copilot_domain::model::llm::InsightExtract;

use super::counters::{Counter, UnderstandingCounters};

/// 抽取路径决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractDecision {
    /// 预筛未命中：不调用网关、不产出草稿。
    Skip,
    /// 能力未配置：走应用层规则兜底。
    RuleFallback,
    /// 正常调用网关。
    Gateway,
}

pub fn decide_extraction(prefilter_hit: bool, endpoint_configured: bool) -> ExtractDecision {
    if !prefilter_hit {
        return ExtractDecision::Skip;
    }
    if !endpoint_configured {
        return ExtractDecision::RuleFallback;
    }
    ExtractDecision::Gateway
}

/// 抽取结果 → 草稿。空来源/空白正文被拒绝并留码；全部被拒时计 `extract_empty`。
pub fn drafts_from_extract(
    extract: &InsightExtract,
    counters: &UnderstandingCounters,
) -> Vec<InsightDraft> {
    let mut drafts = Vec::with_capacity(extract.insights.len());
    for item in &extract.insights {
        if item.sources.is_empty() {
            counters.bump(Counter::SourceMissing);
            continue;
        }
        let Ok(sources) = InsightSources::new(item.sources.clone()) else {
            counters.bump(Counter::SourceMissing);
            continue;
        };
        if item.text.as_str().trim().is_empty() {
            counters.bump(Counter::ExtractEmpty);
            continue;
        }
        drafts.push(InsightDraft {
            kind: item.kind,
            text: item.text.clone(),
            confidence: item.confidence,
            sources,
            fields: InsightFields::default(),
            kind_fallback: false,
        });
    }
    if drafts.is_empty() {
        counters.bump(Counter::ExtractEmpty);
    }
    drafts
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::enums::InsightKind;
    use meeting_copilot_domain::model::llm::InsightItem;
    use meeting_copilot_domain::vo::{Confidence, ShortText, UtteranceRef};

    fn item(sources: Vec<&str>, text: &str) -> InsightItem {
        InsightItem {
            kind: InsightKind::KeyInfo,
            text: ShortText::new(text).expect("text"),
            sources: sources
                .into_iter()
                .map(|name| UtteranceRef::new(name).expect("ref"))
                .collect(),
            confidence: Confidence::new(0.9).expect("conf"),
            decision: None,
            todo: None,
        }
    }

    #[test]
    fn prefilter_miss_skips_gateway_call() {
        assert_eq!(decide_extraction(false, true), ExtractDecision::Skip);
    }

    #[test]
    fn missing_endpoint_falls_back_to_rules() {
        assert_eq!(
            decide_extraction(true, false),
            ExtractDecision::RuleFallback
        );
        assert_eq!(decide_extraction(true, true), ExtractDecision::Gateway);
    }

    #[test]
    fn draft_without_source_is_rejected() {
        let counters = UnderstandingCounters::new();
        let extract = InsightExtract {
            insights: vec![item(Vec::new(), "确认下周上线")],
        };
        assert!(drafts_from_extract(&extract, &counters).is_empty());
        assert_eq!(counters.snapshot().source_missing, 1);
    }

    #[test]
    fn valid_items_become_drafts() {
        let counters = UnderstandingCounters::new();
        let extract = InsightExtract {
            insights: vec![item(vec!["u1"], "确认下周上线")],
        };
        let drafts = drafts_from_extract(&extract, &counters);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].sources.len(), 1);
        assert_eq!(counters.snapshot().source_missing, 0);
    }
}
