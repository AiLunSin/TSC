//! 抽取网关的应用层适配：能力前置、结果转草稿、来源校验。
//!
//! 真正的网络调用由 `StructuredLlmGateway` 承担；本模块保持可离线判定。

use meeting_copilot_domain::model::insight::{InsightDraft, InsightFields, InsightSources};
use meeting_copilot_domain::model::llm::{
    DeadlineMs, FailureClass, InsightExtract, LlmGenerationRequest, LlmTask, OutputBudget,
    RedactedPayload, StructuredBody, TemperatureMilli,
};
use meeting_copilot_domain::port::llm::StructuredLlmGateway;
use meeting_copilot_domain::vo::TaskId;

use super::insight_extractor::prefilter_hit;
use super::rule_fallback;

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

/// 抽取请求的截止上限（毫秒），与应用层 15 s 记账分开。
pub const INSIGHT_DEADLINE_MAX_MS: u64 = 6_000;

/// 构造 `InsightExtract` 请求；空白正文返回 `None`。
pub fn build_request(
    fragment: &crate::understand::types::NormalizedFragment,
    deadline_override_ms: Option<u64>,
) -> Option<LlmGenerationRequest> {
    let text = fragment.text.as_str();
    if text.trim().is_empty() {
        return None;
    }
    let request_id = TaskId::new(format!("insight-{}", fragment.reference.as_str())).ok()?;
    let payload = RedactedPayload::new(text, LlmTask::InsightExtract, 8_000).ok()?;
    let deadline = deadline_override_ms
        .unwrap_or(INSIGHT_DEADLINE_MAX_MS)
        .min(INSIGHT_DEADLINE_MAX_MS);
    LlmGenerationRequest::new(
        request_id,
        payload,
        None,
        OutputBudget::new(512).ok()?,
        TemperatureMilli::new(200).ok()?,
        DeadlineMs::new(deadline).ok()?,
    )
    .ok()
}

/// 串联：预筛 → 能力前置 → 请求 → 网关 → 结果转草稿 → 规则兜底。
pub fn run_extraction(
    fragment: &crate::understand::types::NormalizedFragment,
    gateway: &dyn StructuredLlmGateway,
    counters: &UnderstandingCounters,
    deadline_override_ms: Option<u64>,
) -> Vec<InsightDraft> {
    let fallback = || {
        rule_fallback::produce(fragment)
            .into_iter()
            .collect::<Vec<_>>()
    };
    let hit = prefilter_hit(fragment.text.as_str());
    match decide_extraction(hit, gateway.capability().endpoint_configured) {
        ExtractDecision::Skip => {
            counters.bump(Counter::PrefilterMiss);
            Vec::new()
        }
        ExtractDecision::RuleFallback => {
            counters.bump(Counter::RuleFallbackCount);
            fallback()
        }
        ExtractDecision::Gateway => {
            let Some(request) = build_request(fragment, deadline_override_ms) else {
                counters.bump(Counter::RuleFallbackCount);
                return fallback();
            };
            counters.bump(Counter::GatewayCalls);
            let outcome = match gateway.complete_structured(&request) {
                Ok(outcome) => outcome,
                Err(_) => {
                    counters.bump(Counter::GatewayCooling);
                    counters.bump(Counter::RuleFallbackCount);
                    return fallback();
                }
            };
            if let Some(failure) = outcome.failure {
                if failure == FailureClass::InvalidSchema {
                    counters.bump(Counter::SchemaViolation);
                } else {
                    counters.bump(Counter::GatewayCooling);
                }
                counters.bump(Counter::RuleFallbackCount);
                return fallback();
            }
            match outcome.body {
                Some(StructuredBody::Insight(extract)) => {
                    let drafts = drafts_from_extract(&extract, counters);
                    if drafts.is_empty() {
                        counters.bump(Counter::RuleFallbackCount);
                        fallback()
                    } else {
                        drafts
                    }
                }
                _ => {
                    counters.bump(Counter::SchemaViolation);
                    counters.bump(Counter::RuleFallbackCount);
                    fallback()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::understand::types::NormalizedFragment;
    use meeting_copilot_domain::model::session::TextRange;
    use meeting_copilot_domain::model::utterance::SpeakerRef;
    use meeting_copilot_domain::vo::{SessionId, UtteranceText};

    fn fragment(text: &str) -> NormalizedFragment {
        NormalizedFragment {
            session: SessionId::new("s1").expect("session"),
            reference: UtteranceRef::new("u1").expect("ref"),
            parent_reference: UtteranceRef::new("u1").expect("ref"),
            span: TextRange {
                start_ms: 0,
                end_ms: 1_000,
            },
            text: UtteranceText::new(text).expect("text"),
            speaker: SpeakerRef::unknown(),
            confidence: Confidence::new(0.9).expect("conf"),
            dual_channel: false,
            origin_pair: None,
        }
    }

    #[test]
    fn request_is_built_for_valid_fragment() {
        let request = build_request(&fragment("确认下周灰度上线"), None).expect("request");
        assert_eq!(request.task, LlmTask::InsightExtract);
        assert_eq!(request.deadline.as_millis(), INSIGHT_DEADLINE_MAX_MS);
    }

    #[test]
    fn request_deadline_is_capped_and_overridable() {
        let capped = build_request(&fragment("确认下周灰度上线"), Some(9_000)).expect("request");
        assert_eq!(capped.deadline.as_millis(), INSIGHT_DEADLINE_MAX_MS);
        let short = build_request(&fragment("确认下周灰度上线"), Some(100)).expect("request");
        assert_eq!(short.deadline.as_millis(), 100);
    }
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
