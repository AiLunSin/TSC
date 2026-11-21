//! 通道实例注册表（B2.23.7.1~7.4 的核心）。
//!
//! 每个 `(session, platform, kind)` 独立；迁移规则全部来自 domain。

use std::collections::HashMap;
use std::sync::Arc;

use meeting_copilot_domain::model::{
    transition, ChannelKey, ChannelKind, ChannelState, ChannelStateError, Platform, StateReason,
    TransitionOutcome,
};
use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{MonoMillis, SessionId};

use super::types::{
    AudioDeviceEvent, ChannelObservation, ChannelSnapshot, ChannelSnapshotEntry,
    ObservationOutcome, ObservationPayload, RejectReason, SessionContextView,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelInstance {
    pub key: ChannelKey,
    pub state: ChannelState,
    pub reason: Option<StateReason>,
    pub entered_at: MonoMillis,
    pub changed_at: MonoMillis,
    pub last_seen_at: MonoMillis,
    pub consecutive_read_failures: u8,
    pub last_valid_text_at: Option<MonoMillis>,
    pub stable_since: Option<MonoMillis>,
    pub ocr_failed: u32,
    pub ocr_total: u32,
    pub silence_blocks: u8,
}

impl ChannelInstance {
    fn probing(key: ChannelKey, at: MonoMillis) -> Self {
        Self {
            key,
            state: ChannelState::Probing,
            reason: None,
            entered_at: at,
            changed_at: at,
            last_seen_at: at,
            consecutive_read_failures: 0,
            last_valid_text_at: None,
            stable_since: None,
            ocr_failed: 0,
            ocr_total: 0,
            silence_blocks: 0,
        }
    }
}

struct SessionBound {
    platform: Platform,
    instances: [ChannelInstance; 2],
}

impl SessionBound {
    fn index(kind: ChannelKind) -> usize {
        match kind {
            ChannelKind::Subtitle => 0,
            ChannelKind::Audio => 1,
        }
    }

    fn instance(&self, kind: ChannelKind) -> &ChannelInstance {
        &self.instances[Self::index(kind)]
    }

    fn instance_mut(&mut self, kind: ChannelKind) -> &mut ChannelInstance {
        &mut self.instances[Self::index(kind)]
    }
}

/// 采集审计只接收事件类别与计数，不接收 payload/文本/样本。
pub trait CaptureAuditSink: Send + Sync {
    fn record_rejection(&self, event_kind: &'static str, count: u64);
}

#[derive(Debug, Default)]
pub struct NoopCaptureAudit;

impl CaptureAuditSink for NoopCaptureAudit {
    fn record_rejection(&self, _event_kind: &'static str, _count: u64) {}
}

pub struct ChannelStateRegistry {
    clock: Arc<dyn CaptureClock>,
    audit: Arc<dyn CaptureAuditSink>,
    sessions: HashMap<SessionId, SessionBound>,
    duplicate_instance: u64,
    clock_regression: u64,
    cross_platform_rejected: u64,
}

impl ChannelStateRegistry {
    pub fn new(clock: Arc<dyn CaptureClock>) -> Self {
        Self::with_audit(clock, Arc::new(NoopCaptureAudit))
    }

    pub fn with_audit(clock: Arc<dyn CaptureClock>, audit: Arc<dyn CaptureAuditSink>) -> Self {
        Self {
            clock,
            audit,
            sessions: HashMap::new(),
            duplicate_instance: 0,
            clock_regression: 0,
            cross_platform_rejected: 0,
        }
    }

    fn now(&self) -> Result<MonoMillis, ChannelStateError> {
        self.clock
            .mono_now()
            .map_err(|_| ChannelStateError::ClockRegression {
                seen_max: MonoMillis::ZERO,
                at: MonoMillis::ZERO,
            })
    }

    /// 同会话重复 attach 沿用实例，只累计 `duplicate_instance`。
    pub fn attach_session(
        &mut self,
        context: &SessionContextView,
    ) -> Result<(), ChannelStateError> {
        if let Some(bound) = self.sessions.get(&context.session) {
            if bound.platform != context.platform {
                self.cross_platform_rejected += 1;
                return Err(ChannelStateError::ForeignPlatform {
                    bound: bound.platform,
                    observed: context.platform,
                });
            }
            self.duplicate_instance += 1;
            return Ok(());
        }
        let now = self.now()?;
        let subtitle = ChannelInstance::probing(
            ChannelKey::new(context.platform, ChannelKind::Subtitle),
            now,
        );
        let audio =
            ChannelInstance::probing(ChannelKey::new(context.platform, ChannelKind::Audio), now);
        self.sessions.insert(
            context.session.clone(),
            SessionBound {
                platform: context.platform,
                instances: [subtitle, audio],
            },
        );
        Ok(())
    }

    pub fn detach_session(&mut self, session: &SessionId) {
        self.sessions.remove(session);
    }

    pub fn instance(&self, session: &SessionId, kind: ChannelKind) -> Option<&ChannelInstance> {
        self.sessions.get(session).map(|bound| bound.instance(kind))
    }

    pub fn state(&self, session: &SessionId, kind: ChannelKind) -> Option<ChannelState> {
        self.instance(session, kind).map(|instance| instance.state)
    }

    pub fn snapshot(&self) -> ChannelSnapshot {
        let mut entries: Vec<ChannelSnapshotEntry> = self
            .sessions
            .values()
            .flat_map(|bound| {
                bound.instances.iter().map(|instance| ChannelSnapshotEntry {
                    key: instance.key,
                    state: instance.state,
                    reason: instance.reason,
                    entered_at: instance.entered_at,
                    changed_at: instance.changed_at,
                })
            })
            .collect();
        entries.sort_by_key(|entry| entry.key);
        ChannelSnapshot {
            entries,
            generated_at: self.clock.mono_now().unwrap_or(MonoMillis::ZERO),
        }
    }

    pub fn duplicate_instance(&self) -> u64 {
        self.duplicate_instance
    }

    pub fn clock_regression(&self) -> u64 {
        self.clock_regression
    }

    pub fn cross_platform_rejected(&self) -> u64 {
        self.cross_platform_rejected
    }

    pub fn observe(&mut self, observation: &ChannelObservation) -> ObservationOutcome {
        let Some(session) = observation.session.as_ref() else {
            self.audit.record_rejection("missing_session", 1);
            return ObservationOutcome::Dropped {
                reason: super::types::DropReason::SessionDetached,
            };
        };
        let Some(bound) = self.sessions.get_mut(session) else {
            self.audit.record_rejection("session_not_found", 1);
            return ObservationOutcome::Rejected {
                reason: RejectReason::SessionNotFound,
            };
        };
        if bound.platform != observation.platform {
            self.cross_platform_rejected += 1;
            self.audit.record_rejection("cross_platform", 1);
            return ObservationOutcome::Rejected {
                reason: RejectReason::ForeignPlatform,
            };
        }
        let instance = bound.instance_mut(observation.kind);
        if observation.at < instance.last_seen_at {
            self.clock_regression += 1;
            self.audit.record_rejection("clock_regression", 1);
            return ObservationOutcome::Rejected {
                reason: RejectReason::ClockRegression,
            };
        }
        instance.last_seen_at = observation.at;
        let (target, reason) = match observation.payload {
            ObservationPayload::Probe {
                readable,
                version_supported,
                ocr_only,
            } => {
                if !readable {
                    (ChannelState::Unavailable, StateReason::ContainerMissing)
                } else if ocr_only {
                    (ChannelState::Degraded, StateReason::OcrOnly)
                } else if !version_supported {
                    (ChannelState::Degraded, StateReason::VersionMismatch)
                } else {
                    (ChannelState::Available, StateReason::ProbeCompleted)
                }
            }
            ObservationPayload::ReadFailure => {
                instance.consecutive_read_failures =
                    instance.consecutive_read_failures.saturating_add(1);
                instance.stable_since = None;
                if instance.state == ChannelState::Available
                    && instance.consecutive_read_failures >= 3
                {
                    instance.consecutive_read_failures = 0;
                    (ChannelState::Degraded, StateReason::ConsecutiveReadFailures)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::WindowRebuilt => {
                instance.stable_since = None;
                if instance.state == ChannelState::Available {
                    (ChannelState::Degraded, StateReason::WindowRebuilt)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::Silence { blocks, vad } => {
                instance.silence_blocks = blocks;
                if blocks >= 3 && !vad && instance.state == ChannelState::Available {
                    (ChannelState::Degraded, StateReason::SuspectedSilence)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::OcrStats { failed, total } => {
                instance.ocr_failed = failed;
                instance.ocr_total = total;
                if total > 0
                    && failed.saturating_mul(2) > total
                    && instance.state == ChannelState::Degraded
                {
                    (
                        ChannelState::Unavailable,
                        StateReason::OcrFailureRateAboveThreshold,
                    )
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::Permission { revoked } => {
                if revoked {
                    instance.stable_since = None;
                    (ChannelState::Unavailable, StateReason::PermissionRevoked)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::AudioDevice { event } => {
                instance.stable_since = None;
                match event {
                    AudioDeviceEvent::OpenFailed
                    | AudioDeviceEvent::Unplugged
                    | AudioDeviceEvent::Lost => {
                        (ChannelState::Unavailable, StateReason::DeviceUnplugged)
                    }
                }
            }
            ObservationPayload::UserRetry => {
                if instance.state == ChannelState::Unavailable {
                    (ChannelState::Probing, StateReason::UserRetry)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::OcrRoiChanged => {
                if instance.state == ChannelState::Degraded {
                    (ChannelState::Degraded, StateReason::OcrOnly)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::AudioFallback => {
                if instance.state == ChannelState::Degraded {
                    (ChannelState::Degraded, StateReason::DeviceFallback)
                } else {
                    return ObservationOutcome::NoOp;
                }
            }
            ObservationPayload::ValidText {
                confidence_milli, ..
            } => {
                if confidence_milli.unwrap_or(0) >= 500 {
                    instance.last_valid_text_at = Some(observation.at);
                    instance.consecutive_read_failures = 0;
                    instance.stable_since = Some(observation.at);
                } else {
                    instance.stable_since = None;
                }
                return ObservationOutcome::NoOp;
            }
        };
        match transition(instance.state, target, reason) {
            Ok(TransitionOutcome::Migrated { to, .. })
            | Ok(TransitionOutcome::InternalAdjustment { state: to, .. }) => {
                if instance.state != to {
                    instance.entered_at = observation.at;
                }
                instance.state = to;
                instance.reason = Some(reason);
                instance.changed_at = observation.at;
                if to == ChannelState::Available {
                    instance.stable_since = Some(observation.at);
                    instance.consecutive_read_failures = 0;
                } else {
                    instance.stable_since = None;
                }
                ObservationOutcome::Applied
            }
            Ok(TransitionOutcome::NoOp) => ObservationOutcome::NoOp,
            Ok(TransitionOutcome::Rejected { .. }) => ObservationOutcome::Rejected {
                reason: RejectReason::InvalidTransition,
            },
            Err(_) => ObservationOutcome::Rejected {
                reason: RejectReason::InvalidTransition,
            },
        }
    }
}
