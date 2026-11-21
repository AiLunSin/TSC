//! 滚动摘要的集成用例：触发、版本、保留、空主题、超时与长会话。

use std::sync::Arc;

use meeting_copilot_application::understand::{covered_span, SummaryGuard, UnderstandingCounters};
use meeting_copilot_domain::model::insight::Topic;
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::rolling_summary::{SummaryParse, SummaryStatus};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::{SessionId, UtteranceRef};

fn session() -> SessionId {
    SessionId::new("s1").expect("session")
}

fn reference(name: &str) -> UtteranceRef {
    UtteranceRef::new(name).expect("ref")
}

fn span(start: u64, end: u64) -> TextRange {
    TextRange {
        start_ms: start,
        end_ms: end,
    }
}

fn delta(start: u64, end: u64) -> Vec<(UtteranceRef, TextRange)> {
    vec![(reference("u1"), span(start, end))]
}

fn parsed(label: &str) -> SummaryParse {
    SummaryParse {
        topics: vec![Topic {
            label: label.into(),
            weight: 1.0,
        }],
        entities: Vec::new(),
    }
}

fn guard() -> (SummaryGuard, Arc<UnderstandingCounters>) {
    let counters = Arc::new(UnderstandingCounters::new());
    (SummaryGuard::new(counters.clone()), counters)
}

#[test]
fn refresh_triggers_at_twenty_utterances() {
    let (guard, _counters) = guard();
    let policy = InsightPolicy::default();
    let session = session();
    let mut triggered = None;
    for index in 0..20u64 {
        let result = guard.on_delta(
            &session,
            reference(&format!("u{index}")),
            span(index, index + 1),
            0,
            &policy,
        );
        if index == 19 {
            triggered = result;
        }
    }
    assert_eq!(triggered.map(|delta| delta.len()), Some(20));
}

#[test]
fn refresh_triggers_at_thirty_seconds_once() {
    let (guard, _counters) = guard();
    let policy = InsightPolicy::default();
    let session = session();
    guard.on_delta(&session, reference("u1"), span(0, 1_000), 0, &policy);
    assert!(guard
        .on_delta(
            &session,
            reference("u2"),
            span(1_000, 2_000),
            30_000,
            &policy
        )
        .is_some());
    assert!(guard
        .on_delta(
            &session,
            reference("u3"),
            span(2_000, 3_000),
            30_000,
            &policy
        )
        .is_none());
}

#[test]
fn nineteen_utterances_do_not_trigger() {
    let (guard, _counters) = guard();
    let policy = InsightPolicy::default();
    let session = session();
    let mut triggered = None;
    for index in 0..19u64 {
        triggered = guard.on_delta(
            &session,
            reference(&format!("u{index}")),
            span(index, index + 1),
            0,
            &policy,
        );
    }
    assert!(triggered.is_none());
}

#[test]
fn covered_span_equals_delta_window() {
    let delta = vec![
        (reference("u2"), span(2_000, 3_000)),
        (reference("u1"), span(0, 1_000)),
    ];
    let mut sorted = delta;
    sorted.sort_by_key(|(_, span)| (span.start_ms, span.end_ms));
    assert_eq!(covered_span(&sorted), Some(span(0, 3_000)));
}

#[test]
fn incremental_refresh_consumes_delta_plus_previous() {
    let (guard, _counters) = guard();
    let session = session();
    guard.publish(&session, parsed("灰度"), &delta(0, 1_000));
    let boxes = guard.on_delta(
        &session,
        reference("u2"),
        span(1_000, 2_000),
        0,
        &InsightPolicy::default(),
    );
    assert!(boxes.is_none());
    assert_eq!(guard.current_version(&session), Some(1));
}

#[test]
fn version_is_monotonic_and_dense() {
    let (guard, _counters) = guard();
    let session = session();
    for index in 0..3u64 {
        assert_eq!(
            guard.publish(
                &session,
                parsed(&format!("t{index}")),
                &delta(index * 1_000, index * 1_000 + 1_000)
            ),
            SummaryStatus::Ok
        );
    }
    assert_eq!(guard.current_version(&session), Some(3));
    assert_eq!(guard.previous_version(&session), Some(2));
}

#[test]
fn only_two_versions_are_retained() {
    let (guard, _counters) = guard();
    let session = session();
    for index in 0..4u64 {
        guard.publish(
            &session,
            parsed(&format!("t{index}")),
            &delta(index * 1_000, index * 1_000 + 1_000),
        );
    }
    assert_eq!(guard.retained_versions(&session), 2);
    assert_eq!(guard.previous_version(&session), Some(3));
}

#[test]
fn empty_topics_blocks_publish() {
    let (guard, counters) = guard();
    let status = guard.publish(
        &session(),
        SummaryParse {
            topics: Vec::new(),
            entities: Vec::new(),
        },
        &delta(0, 1_000),
    );
    assert_eq!(status, SummaryStatus::PublishBlocked);
    assert_eq!(counters.snapshot().summary_publish_blocked, 1);
}

#[test]
fn timeout_discards_and_keeps_previous() {
    let (guard, counters) = guard();
    let session = session();
    guard.publish(&session, parsed("灰度"), &delta(0, 1_000));
    guard.timeout(&session);
    assert_eq!(guard.status(&session), Some(SummaryStatus::Timeout));
    assert_eq!(counters.snapshot().summary_timeout, 1);
    assert_eq!(guard.current_version(&session), Some(1));
}

#[test]
fn timeout_does_not_block_admission() {
    let (guard, _counters) = guard();
    let policy = InsightPolicy::default();
    let session = session();
    guard.timeout(&session);
    assert!(guard
        .on_delta(&session, reference("u1"), span(0, 1_000), 0, &policy)
        .is_none());
    assert_eq!(guard.current_version(&session), None);
}

#[test]
fn summary_failures_are_classified_by_cause() {
    let (guard, _counters) = guard();
    let session = session();
    let blocked = guard.publish(
        &session,
        SummaryParse {
            topics: Vec::new(),
            entities: Vec::new(),
        },
        &delta(0, 1_000),
    );
    guard.publish(&session, parsed("灰度"), &delta(1_000, 2_000));
    let out_of_order = guard.publish(&session, parsed("回滚"), &delta(500, 1_500));
    assert_eq!(blocked, SummaryStatus::PublishBlocked);
    assert_eq!(out_of_order, SummaryStatus::OutOfOrder);
    assert_ne!(blocked, out_of_order);
}

#[test]
fn hour_long_session_refreshes_at_least_120_times() {
    let (guard, counters) = guard();
    let session = session();
    for index in 0..120u64 {
        let start = index * 30_000;
        assert_eq!(
            guard.publish(
                &session,
                parsed(&format!("t{index}")),
                &delta(start, start + 1_000)
            ),
            SummaryStatus::Ok
        );
    }
    assert!(counters.snapshot().summary_refresh_ok >= 120);
}
