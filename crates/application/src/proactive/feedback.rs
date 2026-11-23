//! 负反馈加权用例（SPEC 7.1、DESIGN 2.10.1）。
//!
//! 权重只影响 Allow 之后的排序，不参与门控阈值；`Deny` 结果与权重无关。

use std::collections::BTreeMap;

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::{Platform, TriggerKind};
use meeting_copilot_domain::model::feedback::{
    rank_score_milli, FeedbackEvent, FeedbackValue, TopicWeight,
};
use meeting_copilot_domain::port::proactive::FeedbackStore;
use meeting_copilot_domain::vo::{CardRef, ReferenceRef};

pub struct FeedbackRequest {
    pub feedback_id: String,
    pub card_id: CardRef,
    pub dedupe_key: DedupeKey,
    pub topic_cluster_id: String,
    pub source_ref_id: Option<ReferenceRef>,
    pub trigger_kind: TriggerKind,
    pub platform: Platform,
    pub value: FeedbackValue,
    pub occurred_at_ms: i64,
    /// 卡片是否已撤回。已撤回后再反馈只计入统计，不再改变权重或二次撤回。
    pub card_already_withdrawn: bool,
}

impl FeedbackRequest {
    fn source_ref(&self) -> &str {
        self.source_ref_id.as_ref().map_or("", ReferenceRef::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackOutcome {
    pub weight_after_milli: i32,
    pub blocked_until_ms: Option<i64>,
    /// 需要调用方撤回的卡片；仅首次 `reported` 产生。
    pub withdraw: Option<CardRef>,
}

pub struct FeedbackService<'a, S: FeedbackStore> {
    store: &'a S,
}

impl<'a, S: FeedbackStore> FeedbackService<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    pub fn record(&self, request: &FeedbackRequest) -> Result<FeedbackOutcome, CopilotError> {
        let existing = self
            .store
            .weight(&request.topic_cluster_id, request.source_ref())?;
        let base = existing.unwrap_or_else(|| {
            TopicWeight::initial(
                &request.topic_cluster_id,
                request.source_ref(),
                request.occurred_at_ms,
            )
        });

        let (applied, withdraw) = if request.card_already_withdrawn {
            (base, None)
        } else {
            let applied = base.apply(request.value, request.occurred_at_ms);
            let withdraw = if request.value == FeedbackValue::Reported {
                Some(request.card_id.clone())
            } else {
                None
            };
            (applied, withdraw)
        };

        let event = FeedbackEvent {
            feedback_id: request.feedback_id.clone(),
            card_id: request.card_id.clone(),
            dedupe_key: request.dedupe_key.clone(),
            topic_cluster_id: request.topic_cluster_id.clone(),
            source_ref_id: request.source_ref_id.clone(),
            trigger_kind: request.trigger_kind,
            platform: request.platform,
            value: request.value,
            occurred_at_ms: request.occurred_at_ms,
            weight_after_milli: applied.weight_milli,
        };
        event
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;

        self.store.append_event(&event)?;
        self.store.save_weight(&applied)?;

        Ok(FeedbackOutcome {
            weight_after_milli: applied.weight_milli,
            blocked_until_ms: applied.blocked_until_ms,
            withdraw,
        })
    }

    /// 从事件流派生权重，启动或异常恢复时使用。
    pub fn replay(&self) -> Result<Vec<TopicWeight>, CopilotError> {
        let mut weights: BTreeMap<(String, String), TopicWeight> = BTreeMap::new();
        for event in self.store.events()? {
            let key = (
                event.topic_cluster_id.clone(),
                event.source_ref().to_owned(),
            );
            let current = weights.entry(key).or_insert_with(|| {
                TopicWeight::initial(
                    &event.topic_cluster_id,
                    event.source_ref(),
                    event.occurred_at_ms,
                )
            });
            *current = current.apply(event.value, event.occurred_at_ms);
        }
        Ok(weights.into_values().collect())
    }

    pub fn is_suppressed(
        &self,
        topic_cluster_id: &str,
        source_ref_id: &str,
        now_ms: i64,
    ) -> Result<bool, CopilotError> {
        Ok(self
            .store
            .weight(topic_cluster_id, source_ref_id)?
            .is_some_and(|weight| weight.is_suppressed(now_ms)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankInput {
    pub key: String,
    pub base_relevance_milli: i32,
    pub threshold_milli: i32,
    pub weight_milli: i32,
}

/// 先按基础相关度过阈值，再按 `base × weight / 1000` 降序。
/// 阈值判定只读基础相关度，权重不改变准入。
pub fn rank_candidates(inputs: Vec<RankInput>) -> Vec<RankInput> {
    let mut kept: Vec<RankInput> = inputs
        .into_iter()
        .filter(|input| input.base_relevance_milli >= input.threshold_milli)
        .collect();
    kept.sort_by(|left, right| {
        rank_score_milli(right.base_relevance_milli, right.weight_milli)
            .cmp(&rank_score_milli(
                left.base_relevance_milli,
                left.weight_milli,
            ))
            .then_with(|| right.base_relevance_milli.cmp(&left.base_relevance_milli))
            .then_with(|| left.key.cmp(&right.key))
    });
    kept
}
