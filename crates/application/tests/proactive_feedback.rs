//! 负反馈加权用例（SPEC 7.1、DESIGN 2.10.1）。

use std::collections::HashMap;
use std::sync::Mutex;

use meeting_copilot_application::proactive::feedback::{
    rank_candidates, FeedbackRequest, FeedbackService, RankInput,
};
use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::{Platform, TriggerKind};
use meeting_copilot_domain::model::feedback::{
    FeedbackEvent, FeedbackValue, TopicWeight, SUPPRESSION_MS, WEIGHT_MAX_MILLI,
};
use meeting_copilot_domain::port::proactive::FeedbackStore;
use meeting_copilot_domain::vo::CardRef;

#[derive(Default)]
struct FakeFeedbackStore {
    events: Mutex<Vec<FeedbackEvent>>,
    weights: Mutex<HashMap<(String, String), TopicWeight>>,
}

impl FeedbackStore for FakeFeedbackStore {
    fn append_event(&self, event: &FeedbackEvent) -> Result<(), CopilotError> {
        self.events.lock().expect("events").push(event.clone());
        Ok(())
    }

    fn weight(
        &self,
        topic_cluster_id: &str,
        source_ref_id: &str,
    ) -> Result<Option<TopicWeight>, CopilotError> {
        Ok(self
            .weights
            .lock()
            .expect("weights")
            .get(&(topic_cluster_id.to_owned(), source_ref_id.to_owned()))
            .cloned())
    }

    fn save_weight(&self, weight: &TopicWeight) -> Result<(), CopilotError> {
        self.weights.lock().expect("weights").insert(
            (
                weight.topic_cluster_id.clone(),
                weight.source_ref_id.clone(),
            ),
            weight.clone(),
        );
        Ok(())
    }

    fn events(&self) -> Result<Vec<FeedbackEvent>, CopilotError> {
        Ok(self.events.lock().expect("events").clone())
    }
}

fn dedupe(seed: u8) -> DedupeKey {
    DedupeKey::new(format!("{seed:032x}")).expect("dedupe")
}

fn request(seed: u8, value: FeedbackValue, at_ms: i64) -> FeedbackRequest {
    FeedbackRequest {
        feedback_id: format!("fb-{seed}"),
        card_id: CardRef::new(format!("card-{seed}")).expect("card"),
        dedupe_key: dedupe(seed),
        topic_cluster_id: "topic-1".to_owned(),
        source_ref_id: None,
        trigger_kind: TriggerKind::Auto,
        platform: Platform::Zoom,
        value,
        occurred_at_ms: at_ms,
        card_already_withdrawn: false,
    }
}

#[test]
fn two_dismissals_halve_weight() {
    let store = FakeFeedbackStore::default();
    let service = FeedbackService::new(&store);

    let first = service
        .record(&request(1, FeedbackValue::Dismissed, 1_000))
        .expect("first");
    assert_eq!(first.weight_after_milli, 750);
    assert_eq!(first.withdraw, None);

    let second = service
        .record(&request(2, FeedbackValue::Dismissed, 2_000))
        .expect("second");
    assert_eq!(second.weight_after_milli, 500);
    assert!(second.blocked_until_ms.is_none());
}

#[test]
fn reported_withdraws_card_and_suppresses_seven_days() {
    let store = FakeFeedbackStore::default();
    let service = FeedbackService::new(&store);

    let outcome = service
        .record(&request(1, FeedbackValue::Reported, 10_000))
        .expect("reported");
    assert_eq!(outcome.weight_after_milli, WEIGHT_MAX_MILLI);
    assert_eq!(
        outcome.withdraw.map(|card| card.as_str().to_owned()),
        Some("card-1".to_owned())
    );
    assert_eq!(outcome.blocked_until_ms, Some(10_000 + SUPPRESSION_MS));
    assert!(service
        .is_suppressed("topic-1", "", 10_000 + SUPPRESSION_MS - 1)
        .expect("suppressed"));
    assert!(!service
        .is_suppressed("topic-1", "", 10_000 + SUPPRESSION_MS)
        .expect("expired"));

    let mut repeated = request(2, FeedbackValue::Reported, 11_000);
    repeated.card_already_withdrawn = true;
    let again = service.record(&repeated).expect("repeat");
    assert_eq!(again.weight_after_milli, WEIGHT_MAX_MILLI);
    assert_eq!(again.withdraw, None);
}

#[test]
fn replayed_weights_match_live_weights() {
    let store = FakeFeedbackStore::default();
    let service = FeedbackService::new(&store);
    for (seed, value, at_ms) in [
        (1, FeedbackValue::Dismissed, 1_000),
        (2, FeedbackValue::Dismissed, 2_000),
        (3, FeedbackValue::Adopted, 3_000),
    ] {
        service
            .record(&request(seed, value, at_ms))
            .expect("record");
    }

    let live = service
        .is_suppressed("topic-1", "", 0)
        .expect("suppression lookup");
    assert!(!live);
    let replayed = service.replay().expect("replay");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].weight_milli, 600);
}

#[test]
fn weight_never_bypasses_relevance_threshold() {
    let inputs = vec![
        RankInput {
            key: "high-weight".to_owned(),
            base_relevance_milli: 800,
            threshold_milli: 500,
            weight_milli: WEIGHT_MAX_MILLI,
        },
        RankInput {
            key: "low-weight".to_owned(),
            base_relevance_milli: 800,
            threshold_milli: 500,
            weight_milli: 500,
        },
        RankInput {
            key: "below-threshold".to_owned(),
            base_relevance_milli: 400,
            threshold_milli: 500,
            weight_milli: WEIGHT_MAX_MILLI,
        },
    ];

    let ranked = rank_candidates(inputs);
    assert_eq!(
        ranked
            .iter()
            .map(|input| input.key.as_str())
            .collect::<Vec<_>>(),
        vec!["high-weight", "low-weight"]
    );
}
