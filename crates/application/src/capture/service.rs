//! 采集通道服务门面（B2.23.5.1 的最小子集 + B2 离线闭环接线）。
//!
//! `observe` 副作用顺序（契约）：
//! 状态机 → 通道事件发布 → 仲裁/切换记录 → 汇总事件发布 → 守卫缺口判定 → 回补投递。
//! 任何新副作用只能追加在末位。

use std::collections::HashMap;
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
    AsrDeliverySink, ChannelGapLog, ChannelStatusSink, DiagnosticsSink, InsightInvariantPort,
    NoopAsrDelivery, NoopDiagnostics, NoopGapLog, NoopInsight, NoopStatusSink,
};
use super::capture_resume::CaptureResumeLedger;
use super::channel_arbiter::{decide, subtitle_fallback_due, Decision};
use super::channel_state::ChannelStateRegistry;
use super::dedupe_guard::DedupeMergeGuard;
use super::degraded_guard::{DegradedGuard, GapCloseOutcome, GuardTransition};
use super::diagnostics::DiagnosticCounter;
use super::merge_types::{AsrTextEvent, MergedUnit};
use super::status_publisher::StatusPublisher;
use super::subscriptions::{DrainReport, StatusHandler, SubscriptionHandle};
use super::switch_log::{ChannelSwitchLog, SwitchTracker};
use super::types::{
    ActiveChannel, AsrIntentKind, CaptureAvailability, CaptureAvailabilityEvent, CapturePolicy,
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
    diagnostics: Arc<dyn DiagnosticsSink>,
    publisher: Arc<StatusPublisher>,
    guard: Arc<DegradedGuard>,
    merge: Option<Arc<DedupeMergeGuard>>,
    understand: Option<Arc<crate::understand::UnderstandService>>,
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
        Self::with_offline_ports(
            clock,
            switch_log,
            gap_log,
            status_sink,
            asr_sink,
            Arc::new(NoopInsight),
            Arc::new(NoopDiagnostics),
            policy,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_offline_ports(
        clock: Arc<dyn CaptureClock>,
        switch_log: Arc<dyn ChannelSwitchLog>,
        gap_log: Arc<dyn ChannelGapLog>,
        status_sink: Arc<dyn ChannelStatusSink>,
        asr_sink: Arc<dyn AsrDeliverySink>,
        insight: Arc<dyn InsightInvariantPort>,
        diagnostics: Arc<dyn DiagnosticsSink>,
        policy: CapturePolicy,
    ) -> Self {
        let registry_clock = clock.clone();
        let publisher = Arc::new(StatusPublisher::with_diagnostics(
            clock.clone(),
            status_sink.clone(),
            policy,
            diagnostics.clone(),
        ));
        let guard = Arc::new(DegradedGuard::new(
            clock.clone(),
            insight,
            diagnostics.clone(),
            policy,
        ));
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
            diagnostics,
            publisher,
            guard,
            merge: None,
            understand: None,
            policy,
        }
    }

    /// 装配合并编排器（独立构造器，不改既有三个签名）。
    pub fn with_merge(mut self, guard: Arc<DedupeMergeGuard>) -> Self {
        self.merge = Some(guard);
        self
    }

    /// 文本摄入：装配存在时交给编排器；未装配时零执行。
    pub fn ingest_text(&self, side: ChannelKind, event: AsrTextEvent) -> Vec<MergedUnit> {
        match self.merge.as_ref() {
            Some(guard) => guard.push(side, event),
            None => Vec::new(),
        }
    }

    /// 装配理解层（独立构造器，不改既有签名）。
    pub fn with_understand(mut self, service: Arc<crate::understand::UnderstandService>) -> Self {
        self.understand = Some(service);
        self
    }

    /// 合并单元只在此处进入理解层一次；未装配时返回 `Disabled`。
    pub fn admit_merged(
        &self,
        session: &SessionId,
        unit: MergedUnit,
        state: meeting_copilot_domain::model::enums::SessionState,
        platform: Option<meeting_copilot_domain::model::enums::Platform>,
    ) -> crate::understand::UnderstandOutcome {
        let Some(service) = self.understand.as_ref() else {
            return crate::understand::UnderstandOutcome::Disabled;
        };
        service.observe(&crate::understand::UnderstandInput {
            session: session.clone(),
            unit,
            state,
            silence: crate::understand::SilenceSignal::none(),
            session_platform: platform,
        })
    }

    pub fn policy(&self) -> CapturePolicy {
        self.policy
    }

    pub fn diagnostics(&self) -> super::diagnostics::DiagnosticsSnapshot {
        self.diagnostics.snapshot()
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
        self.flush_gaps(session);
        self.publisher.detach(session);
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
                registry.state(session, ChannelKind::Subtitle),
                registry.state(session, ChannelKind::Audio),
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
        drop(trackers);
        self.publish_availability(session, decision, observation.at);
        let transition =
            self.guard
                .on_availability(session, availability_of(decision), observation.at);
        if let Ok(transition) = transition {
            self.apply_transition(session, &transition);
        }
        outcome
    }

    fn publish_availability(&self, session: &SessionId, decision: Decision, at: MonoMillis) {
        let availability = availability_of(decision);
        let (gap_reason, code) = match availability {
            CaptureAvailability::Unavailable { reason, code } => (Some(reason), Some(code)),
            CaptureAvailability::Available { .. } => (None, None),
        };
        self.publisher.publish_availability(
            CaptureAvailabilityEvent {
                availability,
                gap_reason,
                code,
                sequence: 0,
            },
            session,
        );
        let _ = at;
    }

    fn apply_transition(&self, session: &SessionId, transition: &GuardTransition) {
        let GuardTransition::Closed { outcome } = transition else {
            return;
        };
        let GapCloseOutcome::Persisted { record, intent, .. } = outcome else {
            return;
        };
        let active = self.active_channel(session);
        let allowed = self
            .asr_budget
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .entry(session.clone())
            .or_insert_with(AsrBudget::with_default_interval)
            .try_issue(active, AsrIntentKind::GapBackfill, record.end);
        if allowed {
            if let Some(intent) = intent {
                let _ = self.asr_sink.deliver(intent);
            }
        } else {
            self.diagnostics.bump(DiagnosticCounter::BackfillBlocked, 1);
        }
        let mut persisted = record.clone();
        persisted.backfilled = allowed;
        let _ = self.gap_log.append(&persisted);
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
        let wall_at = self
            .clock
            .wall_now()
            .unwrap_or(WallClock::from_unix_millis(0));
        let event = ChannelStatusEvent {
            key: ChannelKey::new(observation.platform, observation.kind),
            state,
            reason: reason.unwrap_or(StateReason::ProbeCompleted),
            sequence: 0,
            determined_at: observation.at,
            wall_at,
        };
        self.publisher.publish_channel(event, session);
    }

    pub fn flush_gaps(&self, session: &SessionId) {
        let at = self.clock.mono_now().unwrap_or(MonoMillis::ZERO);
        let transition = self.guard.force_close(session, at);
        self.apply_transition(session, &transition);
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
        if let Some(guard) = self.merge.as_ref() {
            let _ = guard.advance_to(now);
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
            .guard
            .open_reason(session)
            .unwrap_or(GapReason::BothChannelsUnavailable);
        CaptureAvailability::Unavailable {
            reason,
            code: ErrorCode::AudioCaptureFailed,
        }
    }

    /// 汇总可用性事件（新类型，既有 `availability` 返回类型不变）。
    pub fn availability_event(&self, session: &SessionId) -> CaptureAvailabilityEvent {
        let availability = self.availability(session);
        let (gap_reason, code) = match availability {
            CaptureAvailability::Unavailable { reason, code } => (Some(reason), Some(code)),
            CaptureAvailability::Available { .. } => (None, None),
        };
        CaptureAvailabilityEvent {
            availability,
            gap_reason,
            code,
            sequence: self.publisher.current_sequence(session),
        }
    }

    pub fn snapshot_for(&self, session: &SessionId) -> ChannelSnapshot {
        self.publisher.snapshot_for(session)
    }

    pub fn drain_status(&self, now: MonoMillis) -> DrainReport {
        self.publisher.drain(now)
    }

    pub fn subscribe_status(&self, handler: StatusHandler) -> SubscriptionHandle {
        self.publisher.subscribe(handler)
    }

    pub fn latency(&self) -> super::status_publisher::LatencyStats {
        self.publisher.latency()
    }

    pub fn active_channel(&self, session: &SessionId) -> ActiveChannel {
        self.trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(ActiveChannel::None, SwitchTracker::active)
    }

    /// 快照：注册表为基准，已发布条目按 key 覆盖（既有契约不回退）。
    pub fn snapshot(&self) -> ChannelSnapshot {
        let published = self.publisher.snapshot();
        let mut merged: Vec<super::types::ChannelSnapshotEntry> = self
            .registry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .snapshot()
            .entries;
        for entry in published.entries {
            match merged
                .iter_mut()
                .find(|candidate| candidate.key == entry.key)
            {
                Some(existing) => *existing = entry,
                None => merged.push(entry),
            }
        }
        merged.sort_by_key(|entry| entry.key);
        ChannelSnapshot {
            entries: merged,
            generated_at: published.generated_at,
        }
    }

    pub fn record_missing(&self, session: &SessionId) -> u64 {
        self.trackers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(0, SwitchTracker::record_missing)
    }

    pub fn status_sink(&self) -> &Arc<dyn ChannelStatusSink> {
        &self.status_sink
    }

    /// 只读发布器句柄（测试与装配观察用，不改变既有方法签名）。
    pub fn publisher(&self) -> &Arc<StatusPublisher> {
        &self.publisher
    }
}

fn availability_of(decision: Decision) -> CaptureAvailability {
    if decision.active == ActiveChannel::None {
        CaptureAvailability::Unavailable {
            reason: decision.gap.unwrap_or(GapReason::BothChannelsUnavailable),
            code: ErrorCode::AudioCaptureFailed,
        }
    } else {
        CaptureAvailability::Available {
            channel: decision.active,
        }
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
