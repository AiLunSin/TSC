//! 滚动摘要的发布裁决：版本单调、空主题阻止、乱序阻止、超时丢弃。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use meeting_copilot_domain::model::insight::RollingSummary;
use meeting_copilot_domain::model::rolling_summary::{
    SummaryParse, SummaryStatus, SummaryVersionPair,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::{SessionId, UtteranceRef};

use super::counters::{Counter, UnderstandingCounters};
use super::rolling_summary::{covered_span, SummaryWindow};

#[derive(Default)]
struct GuardState {
    window: SummaryWindow,
    pair: Option<SummaryVersionPair>,
    inflight: bool,
    status: Option<SummaryStatus>,
}

/// 摘要守护：窗口累积、触发互斥、发布裁决与三态。
pub struct SummaryGuard {
    states: Mutex<HashMap<SessionId, GuardState>>,
    counters: Arc<UnderstandingCounters>,
}

impl SummaryGuard {
    pub fn new(counters: Arc<UnderstandingCounters>) -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            counters,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<SessionId, GuardState>> {
        self.states
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// 累积增量；满足触发条件且不在飞行中时返回本次 delta。
    pub fn on_delta(
        &self,
        session: &SessionId,
        reference: UtteranceRef,
        span: TextRange,
        now_ms: u64,
        policy: &meeting_copilot_domain::model::insight_policy::InsightPolicy,
    ) -> Option<Vec<(UtteranceRef, TextRange)>> {
        let mut states = self.lock();
        let state = states.entry(session.clone()).or_default();
        state.window.accumulate(reference, span, now_ms);
        if state.inflight || !state.window.should_trigger(policy, now_ms) {
            return None;
        }
        state.inflight = true;
        Some(state.window.take())
    }

    /// 超时：版本不变，状态记 `Timeout`，计一次。
    pub fn timeout(&self, session: &SessionId) {
        let mut states = self.lock();
        let state = states.entry(session.clone()).or_default();
        state.inflight = false;
        state.status = Some(SummaryStatus::Timeout);
        self.counters.bump(Counter::SummaryTimeout);
    }

    /// 发布裁决：空主题 / 乱序不发布；成功则版本 +1 并保留两版。
    pub fn publish(
        &self,
        session: &SessionId,
        parsed: SummaryParse,
        delta: &[(UtteranceRef, TextRange)],
    ) -> SummaryStatus {
        let Some(covered) = covered_span(delta) else {
            let mut states = self.lock();
            let state = states.entry(session.clone()).or_default();
            state.inflight = false;
            state.status = Some(SummaryStatus::PublishBlocked);
            self.counters.bump(Counter::SummaryPublishBlocked);
            return SummaryStatus::PublishBlocked;
        };

        let mut states = self.lock();
        let state = states.entry(session.clone()).or_default();

        if parsed.topics.is_empty() {
            state.inflight = false;
            state.status = Some(SummaryStatus::PublishBlocked);
            self.counters.bump(Counter::SummaryPublishBlocked);
            return SummaryStatus::PublishBlocked;
        }

        if let Some(pair) = state.pair.as_ref() {
            if covered.start_ms < pair.current.covered.start_ms {
                state.inflight = false;
                state.status = Some(SummaryStatus::OutOfOrder);
                self.counters.bump(Counter::SummaryOutOfOrder);
                return SummaryStatus::OutOfOrder;
            }
        }

        let next = RollingSummary {
            version: state
                .pair
                .as_ref()
                .map(|pair| pair.current.version.saturating_add(1))
                .unwrap_or(1),
            covered,
            topics: parsed.topics,
            entities: parsed.entities,
        };

        let published = match state.pair.as_mut() {
            Some(pair) => pair.publish(next).is_ok(),
            None => {
                state.pair = Some(SummaryVersionPair::new(next));
                true
            }
        };

        state.inflight = false;
        if published {
            state.status = Some(SummaryStatus::Ok);
            self.counters.bump(Counter::SummaryRefreshOk);
            SummaryStatus::Ok
        } else {
            state.status = Some(SummaryStatus::OutOfOrder);
            self.counters.bump(Counter::SummaryOutOfOrder);
            SummaryStatus::OutOfOrder
        }
    }

    pub fn status(&self, session: &SessionId) -> Option<SummaryStatus> {
        self.lock().get(session).and_then(|state| state.status)
    }

    pub fn current_version(&self, session: &SessionId) -> Option<u32> {
        self.lock()
            .get(session)
            .and_then(|state| state.pair.as_ref().map(|pair| pair.current.version))
    }

    pub fn previous_version(&self, session: &SessionId) -> Option<u32> {
        self.lock().get(session).and_then(|state| {
            state
                .pair
                .as_ref()
                .and_then(|pair| pair.previous.as_ref().map(|previous| previous.version))
        })
    }

    pub fn retained_versions(&self, session: &SessionId) -> u8 {
        self.lock()
            .get(session)
            .and_then(|state| {
                state
                    .pair
                    .as_ref()
                    .map(SummaryVersionPair::retained_versions)
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::insight::{Entity, Topic};
    use meeting_copilot_domain::model::insight_policy::InsightPolicy;

    fn session() -> SessionId {
        SessionId::new("s1").expect("session")
    }

    fn delta(start: u64, end: u64) -> Vec<(UtteranceRef, TextRange)> {
        vec![(
            UtteranceRef::new("u1").expect("ref"),
            TextRange {
                start_ms: start,
                end_ms: end,
            },
        )]
    }

    fn parsed(label: &str) -> SummaryParse {
        SummaryParse {
            topics: vec![Topic {
                label: label.into(),
                weight: 1.0,
            }],
            entities: vec![Entity {
                name: "张三".into(),
                normalized: "张三".into(),
            }],
        }
    }

    fn guard() -> (SummaryGuard, Arc<UnderstandingCounters>) {
        let counters = Arc::new(UnderstandingCounters::new());
        (SummaryGuard::new(counters.clone()), counters)
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
        assert_eq!(guard.current_version(&session()), None);
    }

    #[test]
    fn out_of_order_window_is_not_published() {
        let (guard, counters) = guard();
        assert_eq!(
            guard.publish(&session(), parsed("灰度"), &delta(1_000, 2_000)),
            SummaryStatus::Ok
        );
        assert_eq!(
            guard.publish(&session(), parsed("回滚"), &delta(500, 1_500)),
            SummaryStatus::OutOfOrder
        );
        assert_eq!(counters.snapshot().summary_out_of_order, 1);
        assert_eq!(guard.current_version(&session()), Some(1));
    }

    #[test]
    fn version_is_monotonic_and_dense() {
        let (guard, _counters) = guard();
        for (index, start) in [0u64, 1_000, 2_000].iter().enumerate() {
            assert_eq!(
                guard.publish(
                    &session(),
                    parsed(&format!("t{index}")),
                    &delta(*start, *start + 1_000)
                ),
                SummaryStatus::Ok
            );
        }
        assert_eq!(guard.current_version(&session()), Some(3));
        assert_eq!(guard.previous_version(&session()), Some(2));
    }

    #[test]
    fn only_two_versions_are_retained() {
        let (guard, _counters) = guard();
        for index in 0..4u64 {
            guard.publish(
                &session(),
                parsed(&format!("t{index}")),
                &delta(index * 1_000, index * 1_000 + 1_000),
            );
        }
        assert_eq!(guard.current_version(&session()), Some(4));
        assert_eq!(guard.previous_version(&session()), Some(3));
        assert_eq!(guard.retained_versions(&session()), 2);
    }

    #[test]
    fn timeout_discards_and_keeps_previous() {
        let (guard, counters) = guard();
        guard.publish(&session(), parsed("灰度"), &delta(0, 1_000));
        guard.timeout(&session());
        assert_eq!(guard.status(&session()), Some(SummaryStatus::Timeout));
        assert_eq!(counters.snapshot().summary_timeout, 1);
        assert_eq!(guard.current_version(&session()), Some(1));
    }

    #[test]
    fn trigger_is_serialized_and_consuming() {
        let (guard, _counters) = guard();
        let policy = InsightPolicy::default();
        let session = session();
        let mut triggered = None;
        for index in 0..20u64 {
            let result = guard.on_delta(
                &session,
                UtteranceRef::new(format!("u{index}")).expect("ref"),
                TextRange {
                    start_ms: index,
                    end_ms: index + 1,
                },
                0,
                &policy,
            );
            if index == 19 {
                triggered = result;
            }
        }
        assert_eq!(triggered.map(|delta| delta.len()), Some(20));
        assert!(guard
            .on_delta(
                &session,
                UtteranceRef::new("u21").expect("ref"),
                TextRange {
                    start_ms: 21,
                    end_ms: 22,
                },
                0,
                &policy,
            )
            .is_none());
    }
}
