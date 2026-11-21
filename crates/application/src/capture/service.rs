//! 采集通道服务门面（B2.23.5.1 的最小子集）。
//!
//! 已装配：注册表、仲裁决策、切换记录、gap/status/asr 端口。
//! 尚未：同毫秒合并进入真实写入、完整 `advance_to`、推送订阅。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use meeting_copilot_domain::error::ErrorCode;
use meeting_copilot_domain::model::{
    ChannelKey, ChannelKind, ChannelPreference, ChannelState, ChannelStateError, GapReason,
    SessionState, StateReason, SwitchReason,
};
use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};

use super::capture_budget::AsrBudget;
use super::capture_ports::{
    AsrDeliverySink, ChannelGapLog, ChannelStatusSink, NoopAsrDelivery, NoopGapLog, NoopStatusSink,
};
use super::capture_resume::CaptureResumeLedger;
use super::channel_arbiter::{decide, subtitle_fallback_due, Decision};
use super::channel_state::ChannelStateRegistry;
use super::switch_log::{ChannelSwitchLog, SwitchTracker};
use super::types::{
    ActiveChannel, AsrIntent, AsrIntentKind, CaptureAvailability, CaptureGapRecord, CapturePolicy,
    ChannelObservation, ChannelSnapshot, ChannelStatusEvent, ObservationOutcome,
    ObservationPayload, RejectReason, SessionContextView,
};

pub struct CaptureChannelService {
    clock: Arc<dyn CaptureClock>,
    registry: Mutex<ChannelStateRegistry>,
    contexts: Mutex<HashMap<SessionId, SessionContextView>>,
    trackers: Mutex<HashMap<SessionId, SwitchTracker>>,
    resume: Mutex<HashMap<SessionId, CaptureResumeLedger>>,
    asr_budget: Mutex<HashMap<SessionId, AsrBudget>>,
    switch_log: Arc<dyn ChannelSwitchLog>,
    gap_log: Arc<dyn ChannelGapLog>,
    status_sink: Arc<dyn ChannelStatusSink>,
    asr_sink: Arc<dyn AsrDeliverySink>,
    status_seq: AtomicU64,
    gap_start: Mutex<HashMap<SessionId, (MonoMillis, GapReason)>>,
    policy: CapturePolicy,
}

impl CaptureChannelService {
    pub fn new(
        clock: Arc<dyn CaptureClock>,
        switch_log: Arc<dyn ChannelSwitchLog>,
        policy: CapturePolicy,
    ) -> Self {
        Self::with_ports(
            clock,
            switch_log,
            Arc::new(NoopGapLog),
            Arc::new(NoopStatusSink),
            Arc::new(NoopAsrDelivery),
            policy,
        )
    }

    pub fn with_ports(
        clock: Arc<dyn CaptureClock>,
        switch_log: Arc<dyn ChannelSwitchLog>,
        gap_log: Arc<dyn ChannelGapLog>,
        status_sink: Arc<dyn ChannelStatusSink>,
        asr_sink: Arc<dyn AsrDeliverySink>,
        policy: CapturePolicy,
    ) -> Self {
        let registry_clock = clock.clone();
        Self {
            clock,
            registry: Mutex::new(ChannelStateRegistry::new(registry_clock)),
            contexts: Mutex::new(HashMap::new()),
            trackers: Mutex::new(HashMap::new()),
            resume: Mutex::new(HashMap::new()),
            asr_budget: Mutex::new(HashMap::new()),
            switch_log,
            gap_log,
            status_sink,
            asr_sink,
            status_seq: AtomicU64::new(0),
            gap_start: Mutex::new(HashMap::new()),
            policy,
        }
    }

    pub fn policy(&self) -> CapturePolicy {
        self.policy
    }

    pub fn attach_session(&self, context: SessionContextView) -> Result<(), ChannelStateError> {
        self.registry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .attach_session(&context)?;
        self.contexts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .insert(context.session.clone(), context.clone());
        self.trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .entry(context.session.clone())
            .or_default();
        self.resume
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .entry(context.session.clone())
            .or_default();
        self.asr_budget
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .entry(context.session)
            .or_insert_with(AsrBudget::with_default_interval);
        Ok(())
    }

    pub fn detach_session(&self, session: &SessionId) {
        if let Some((start, reason)) = self
            .gap_start
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session)
        {
            let end = self.clock.mono_now().unwrap_or(MonoMillis::ZERO);
            let record = CaptureGapRecord {
                session: session.clone(),
                start,
                end,
                reason,
                backfilled: false,
            };
            let _ = self.gap_log.append(&record);
        }
        self.registry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .detach_session(session);
        self.contexts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
        self.trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
        self.resume
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
        self.asr_budget
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
    }

    pub fn observe(&self, observation: &ChannelObservation) -> ObservationOutcome {
        let outcome = self
            .registry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .observe(observation);
        if !matches!(
            outcome,
            ObservationOutcome::Applied | ObservationOutcome::NoOp
        ) {
            return outcome;
        }
        if outcome == ObservationOutcome::Applied {
            self.publish_status(observation);
        }
        let Some(session) = observation.session.as_ref() else {
            return outcome;
        };
        let Some(context) = self
            .contexts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .cloned()
        else {
            return outcome;
        };
        let (subtitle, audio) = {
            let registry = self
                .registry
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            (
                registry.state(
                    session,
                    meeting_copilot_domain::model::ChannelKind::Subtitle,
                ),
                registry.state(session, meeting_copilot_domain::model::ChannelKind::Audio),
            )
        };
        let Some(subtitle) = subtitle else {
            return outcome;
        };
        let Some(audio) = audio else {
            return outcome;
        };
        let decision = decide(context.channel_preference, subtitle, audio);
        let wall_at = self
            .clock
            .wall_now()
            .unwrap_or(WallClock::from_unix_millis(0));
        let mut trackers = self
            .trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let tracker = trackers.entry(session.clone()).or_default();
        let before = tracker.active();
        let reason = switch_reason(before, decision.active, observation.payload);
        tracker.on_decision(
            session,
            decision,
            reason,
            observation.at,
            wall_at,
            self.switch_log.as_ref(),
        );
        self.handle_gap(session, decision, observation.at);
        outcome
    }

    fn publish_status(&self, observation: &ChannelObservation) {
        let Some(session) = observation.session.as_ref() else {
            return;
        };
        let (state, reason) = {
            let registry = self
                .registry
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            registry
                .instance(session, observation.kind)
                .map(|instance| (instance.state, instance.reason))
                .unwrap_or((ChannelState::Probing, None))
        };
        let sequence = self.status_seq.fetch_add(1, Ordering::SeqCst);
        let wall_at = self
            .clock
            .wall_now()
            .unwrap_or(WallClock::from_unix_millis(0));
        let event = ChannelStatusEvent {
            key: ChannelKey::new(observation.platform, observation.kind),
            state,
            reason: reason.unwrap_or(StateReason::ProbeCompleted),
            sequence,
            determined_at: observation.at,
            wall_at,
        };
        let _ = self.status_sink.publish(&event);
    }

    fn handle_gap(&self, session: &SessionId, decision: Decision, at: MonoMillis) {
        let mut gaps = self
            .gap_start
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if decision.active == ActiveChannel::None {
            if let Some(reason) = decision.gap {
                gaps.entry(session.clone()).or_insert((at, reason));
                self.resume
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .entry(session.clone())
                    .or_default()
                    .begin_gap(at);
            }
            return;
        }
        if let Some((start, reason)) = gaps.remove(session) {
            let allowed = self
                .asr_budget
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .entry(session.clone())
                .or_insert_with(AsrBudget::with_default_interval)
                .try_issue(decision.active, AsrIntentKind::GapBackfill, at);
            if allowed {
                let chunk_count = u32::try_from(
                    at.as_millis()
                        .saturating_sub(start.as_millis())
                        .div_ceil(400)
                        .max(1),
                )
                .unwrap_or(u32::MAX);
                let intent = AsrIntent {
                    session: session.clone(),
                    kind: AsrIntentKind::GapBackfill,
                    window_start: start,
                    window_end: at,
                    chunk_count,
                };
                let _ = self.asr_sink.deliver(&intent);
            }
            let record = CaptureGapRecord {
                session: session.clone(),
                start,
                end: at,
                reason,
                backfilled: allowed,
            };
            let _ = self.gap_log.append(&record);
            self.resume
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .entry(session.clone())
                .or_default()
                .end_gap();
        }
    }

    pub fn record_point(&self, session: &SessionId, point_id: &str, at: MonoMillis) -> bool {
        self.resume
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get_mut(session)
            .is_some_and(|ledger| ledger.record_point(point_id, at))
    }

    pub fn delivered_points(&self, session: &SessionId) -> usize {
        self.resume
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(0, CaptureResumeLedger::delivered_count)
    }

    pub fn session_state(&self, session: &SessionId) -> Option<SessionState> {
        self.contexts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map(|context| context.session_state)
    }

    pub fn asr_sink(&self) -> &Arc<dyn AsrDeliverySink> {
        &self.asr_sink
    }

    /// 5s 无有效文本时回退：复用 Probe 迁移 + 仲裁，不做真实睡眠。
    pub fn advance_to(&self, now: MonoMillis) -> u32 {
        let contexts: Vec<SessionContextView> = self
            .contexts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .values()
            .cloned()
            .collect();
        let mut applied = 0;
        for context in contexts {
            let instance = {
                let registry = self
                    .registry
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                registry
                    .instance(&context.session, ChannelKind::Subtitle)
                    .copied()
            };
            let Some(instance) = instance else {
                continue;
            };
            let last_valid = instance.last_valid_text_at.or(Some(instance.entered_at));
            if !subtitle_fallback_due(now, last_valid, &self.policy, instance.state) {
                continue;
            }
            let observation = ChannelObservation {
                session: Some(context.session.clone()),
                platform: context.platform,
                kind: ChannelKind::Subtitle,
                at: now,
                payload: ObservationPayload::Probe {
                    readable: false,
                    version_supported: false,
                    ocr_only: false,
                },
            };
            if matches!(self.observe(&observation), ObservationOutcome::Applied) {
                applied += 1;
            }
        }
        applied
    }

    pub fn set_preference(
        &self,
        session: &SessionId,
        next: ChannelPreference,
    ) -> ObservationOutcome {
        {
            let mut contexts = self
                .contexts
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let Some(context) = contexts.get_mut(session) else {
                return ObservationOutcome::Rejected {
                    reason: RejectReason::SessionNotFound,
                };
            };
            context.channel_preference = next;
        }
        let (subtitle, audio) = {
            let registry = self
                .registry
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            (
                registry.state(session, ChannelKind::Subtitle),
                registry.state(session, ChannelKind::Audio),
            )
        };
        let (Some(subtitle), Some(audio)) = (subtitle, audio) else {
            return ObservationOutcome::NoOp;
        };
        let decision = decide(next, subtitle, audio);
        let at = self.clock.mono_now().unwrap_or(MonoMillis::ZERO);
        let wall_at = self
            .clock
            .wall_now()
            .unwrap_or(WallClock::from_unix_millis(0));
        let mut trackers = self
            .trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let tracker = trackers.entry(session.clone()).or_default();
        let changed = tracker
            .on_decision(
                session,
                decision,
                SwitchReason::PreferenceChanged,
                at,
                wall_at,
                self.switch_log.as_ref(),
            )
            .is_some();
        if changed {
            ObservationOutcome::Applied
        } else {
            ObservationOutcome::NoOp
        }
    }

    pub fn availability(&self, session: &SessionId) -> CaptureAvailability {
        let active = self.active_channel(session);
        if active != ActiveChannel::None {
            return CaptureAvailability::Available { channel: active };
        }
        let reason = self
            .gap_start
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(GapReason::BothChannelsUnavailable, |(_, reason)| *reason);
        CaptureAvailability::Unavailable {
            reason,
            code: ErrorCode::AudioCaptureFailed,
        }
    }

    pub fn active_channel(&self, session: &SessionId) -> ActiveChannel {
        self.trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(ActiveChannel::None, SwitchTracker::active)
    }

    pub fn snapshot(&self) -> ChannelSnapshot {
        self.registry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .snapshot()
    }

    pub fn record_missing(&self, session: &SessionId) -> u64 {
        self.trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(0, SwitchTracker::record_missing)
    }
}

fn switch_reason(
    before: ActiveChannel,
    after: ActiveChannel,
    payload: ObservationPayload,
) -> SwitchReason {
    if before == ActiveChannel::None {
        return SwitchReason::SessionInitial;
    }
    if before == ActiveChannel::Audio && after == ActiveChannel::Subtitle {
        return SwitchReason::SubtitleStableRecovered;
    }
    if before == ActiveChannel::Subtitle && after == ActiveChannel::Audio {
        return match payload {
            ObservationPayload::ReadFailure => SwitchReason::SubtitleReadFailure,
            _ => SwitchReason::SubtitleNoValidText,
        };
    }
    SwitchReason::PreferenceChanged
}
