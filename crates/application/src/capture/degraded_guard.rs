//! 双通道失效保护状态机（AC-B2-OC-C01~C09）。
//!
//! 依赖集仅含只读要点端口、诊断计数与注入时钟；**不含任何仓储写 trait**（I5）。
//! 缺口「合并 → 门限」在此单点决策（K1），本模块不执行任何 I/O。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use meeting_copilot_domain::model::GapReason;
use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{MonoMillis, SessionId};

use super::capture_gap_gate::{merge_for_persist, persist_gate, resume_boundary, GateDecision};
use super::capture_ports::{DiagnosticsSink, InsightInvariantPort};
use super::diagnostics::DiagnosticCounter;
use super::types::{
    AsrIntent, AsrIntentKind, CaptureAvailability, CaptureGapRecord, CapturePolicy, ResumeBoundary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GuardError {
    #[error("注入时钟不可用")]
    ClockUnavailable,
    #[error("观测时刻回退：本次 {evidence:?}，已见 {last_seen:?}")]
    ClockRegression {
        evidence: MonoMillis,
        last_seen: MonoMillis,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapCloseOutcome {
    Persisted {
        record: CaptureGapRecord,
        resume: ResumeBoundary,
        intent: Option<AsrIntent>,
    },
    DroppedShort {
        duration_ms: u64,
    },
    NoOpenGap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardTransition {
    Opened { first_push: bool },
    Sustained,
    Closed { outcome: GapCloseOutcome },
    Reopened,
}

#[derive(Debug, Clone, Copy)]
struct OpenGap {
    start: MonoMillis,
    reason: GapReason,
}

#[derive(Default)]
struct GuardSession {
    open: Option<OpenGap>,
    pending: Option<CaptureGapRecord>,
    last_seen: Option<MonoMillis>,
}

pub struct DegradedGuard {
    clock: Arc<dyn CaptureClock>,
    insight: Arc<dyn InsightInvariantPort>,
    diagnostics: Arc<dyn DiagnosticsSink>,
    policy: CapturePolicy,
    sessions: Mutex<HashMap<SessionId, GuardSession>>,
}

impl DegradedGuard {
    pub fn new(
        clock: Arc<dyn CaptureClock>,
        insight: Arc<dyn InsightInvariantPort>,
        diagnostics: Arc<dyn DiagnosticsSink>,
        policy: CapturePolicy,
    ) -> Self {
        Self {
            clock,
            insight,
            diagnostics,
            policy,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// 观测时刻回退检测；合法推进更新已见最大时刻。
    pub fn on_observation_time(
        &self,
        session: &SessionId,
        evidence_at: MonoMillis,
    ) -> Result<(), GuardError> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let entry = sessions.entry(session.clone()).or_default();
        if let Some(last_seen) = entry.last_seen {
            if evidence_at < last_seen {
                self.diagnostics.bump(DiagnosticCounter::ClockRegression, 1);
                return Err(GuardError::ClockRegression {
                    evidence: evidence_at,
                    last_seen,
                });
            }
        }
        entry.last_seen = Some(evidence_at);
        Ok(())
    }

    /// 汇总可用性唯一入口：开启 / 持续 / 关闭 / 重开。
    pub fn on_availability(
        &self,
        session: &SessionId,
        availability: CaptureAvailability,
        evidence_at: MonoMillis,
    ) -> Result<GuardTransition, GuardError> {
        self.on_observation_time(session, evidence_at)?;
        match availability {
            CaptureAvailability::Unavailable { reason, .. } => {
                Ok(self.on_unavailable(session, reason, evidence_at))
            }
            CaptureAvailability::Available { .. } => Ok(self.on_available(session, evidence_at)),
        }
    }

    fn on_unavailable(
        &self,
        session: &SessionId,
        reason: GapReason,
        evidence_at: MonoMillis,
    ) -> GuardTransition {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let entry = sessions.entry(session.clone()).or_default();
        if entry.open.is_some() {
            self.check_insight(session);
            return GuardTransition::Sustained;
        }
        let reopen_start = entry
            .pending
            .as_ref()
            .filter(|pending| {
                evidence_at.as_millis()
                    <= pending
                        .end
                        .as_millis()
                        .saturating_add(self.policy.gap_reopen_merge_ms)
            })
            .map(|pending| pending.start);
        if let Some(start) = reopen_start {
            entry.open = Some(OpenGap { start, reason });
            self.diagnostics.bump(DiagnosticCounter::GapReopened, 1);
            return GuardTransition::Reopened;
        }
        entry.open = Some(OpenGap {
            start: evidence_at,
            reason,
        });
        self.diagnostics.bump(DiagnosticCounter::GapOpened, 1);
        self.check_insight(session);
        GuardTransition::Opened { first_push: true }
    }

    fn on_available(&self, session: &SessionId, evidence_at: MonoMillis) -> GuardTransition {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let entry = sessions.entry(session.clone()).or_default();
        let Some(open) = entry.open.take() else {
            return GuardTransition::Closed {
                outcome: GapCloseOutcome::NoOpenGap,
            };
        };
        let incoming = CaptureGapRecord {
            session: session.clone(),
            start: open.start,
            end: evidence_at,
            reason: open.reason,
            backfilled: false,
        };
        let pending = entry.pending.take();
        let (merged, discarded_pending) = match merge_for_persist(
            pending.clone(),
            incoming.clone(),
            self.policy.gap_reopen_merge_ms,
        ) {
            Some(merged) => (merged, false),
            None => (incoming.clone(), pending.is_some()),
        };
        if discarded_pending {
            self.diagnostics.bump(DiagnosticCounter::ShortGapDropped, 1);
        }
        let duration = merged.end.saturating_sub(merged.start);

        match persist_gate(merged.start, merged.end, self.policy.gap_min_ms) {
            GateDecision::Persist => {
                let resume_from =
                    resume_boundary(merged.end, merged.start, self.policy.backfill_max_ms);
                let resume = ResumeBoundary {
                    session: session.clone(),
                    resume_from,
                    gap_end: merged.end,
                };
                let intent = AsrIntent {
                    session: session.clone(),
                    kind: AsrIntentKind::GapBackfill,
                    window_start: resume_from,
                    window_end: merged.end,
                    chunk_count: u32::try_from(
                        merged
                            .end
                            .as_millis()
                            .saturating_sub(resume_from.as_millis())
                            .div_ceil(400)
                            .max(1),
                    )
                    .unwrap_or(u32::MAX),
                };
                self.diagnostics.bump(DiagnosticCounter::GapPersisted, 1);
                GuardTransition::Closed {
                    outcome: GapCloseOutcome::Persisted {
                        record: merged,
                        resume,
                        intent: Some(intent),
                    },
                }
            }
            GateDecision::DropShort => {
                self.diagnostics.bump(DiagnosticCounter::ShortGapDropped, 1);
                entry.pending = Some(merged);
                GuardTransition::Closed {
                    outcome: GapCloseOutcome::DroppedShort {
                        duration_ms: duration,
                    },
                }
            }
        }
    }

    fn check_insight(&self, session: &SessionId) {
        if self.insight.read_insight_ids(session).is_err() {
            self.diagnostics
                .bump(DiagnosticCounter::InsightCheckSkipped, 1);
        }
    }

    /// 待落盘/待合并记录（只读）。
    pub fn pending(&self, session: &SessionId) -> Option<CaptureGapRecord> {
        self.sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .and_then(|entry| entry.pending.clone())
    }

    /// 当前开启缺口的原因（只读）。
    pub fn open_reason(&self, session: &SessionId) -> Option<GapReason> {
        self.sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .and_then(|entry| entry.open)
            .map(|open| open.reason)
    }

    /// 会话结束时强制关闭未结算缺口；未达门限的暂存只计数后丢弃。
    pub fn force_close(&self, session: &SessionId, at: MonoMillis) -> GuardTransition {
        let had_open = self.open_reason(session).is_some();
        let transition = self.on_available(session, at);
        if !had_open {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if let Some(entry) = sessions.get_mut(session) {
                entry.pending = None;
            }
            return transition;
        }
        if matches!(
            transition,
            GuardTransition::Closed {
                outcome: GapCloseOutcome::DroppedShort { .. }
            }
        ) {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if let Some(entry) = sessions.get_mut(session) {
                entry.pending = None;
            }
        }
        transition
    }

    pub fn diagnostics(&self) -> super::diagnostics::DiagnosticsSnapshot {
        self.diagnostics.snapshot()
    }

    /// 只读时钟查询，供装配层构造证据时刻；不产生状态变化。
    pub fn now(&self) -> Result<MonoMillis, GuardError> {
        self.clock
            .mono_now()
            .map_err(|_| GuardError::ClockUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::capture_ports::{NoopDiagnostics, NoopInsight};
    use meeting_copilot_domain::vo::{ClockError, WallClock};

    struct FixedClock(MonoMillis);

    impl CaptureClock for FixedClock {
        fn mono_now(&self) -> Result<MonoMillis, ClockError> {
            Ok(self.0)
        }

        fn wall_now(&self) -> Result<WallClock, ClockError> {
            Ok(WallClock::from_unix_millis(0))
        }
    }

    fn guard() -> DegradedGuard {
        DegradedGuard::new(
            Arc::new(FixedClock(MonoMillis::ZERO)),
            Arc::new(NoopInsight),
            Arc::new(NoopDiagnostics),
            CapturePolicy::default(),
        )
    }

    /// AC-B2-OC-C08：first_push 仅在缺口首次为 true。
    #[test]
    fn first_push_is_true_only_for_gap_open() {
        let guard = guard();
        let session = SessionId::new("s1").expect("session");
        assert_eq!(
            guard
                .on_availability(
                    &session,
                    CaptureAvailability::Unavailable {
                        reason: GapReason::BothChannelsUnavailable,
                        code: meeting_copilot_domain::error::ErrorCode::AudioCaptureFailed,
                    },
                    MonoMillis::from_millis(10),
                )
                .expect("open"),
            GuardTransition::Opened { first_push: true }
        );
        assert_eq!(
            guard
                .on_availability(
                    &session,
                    CaptureAvailability::Unavailable {
                        reason: GapReason::PreferenceRestricted,
                        code: meeting_copilot_domain::error::ErrorCode::AudioCaptureFailed,
                    },
                    MonoMillis::from_millis(20),
                )
                .expect("sustain"),
            GuardTransition::Sustained
        );
    }
}
