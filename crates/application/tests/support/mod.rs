#![allow(dead_code)]

//! 应用层测试替身：可手动推进的时钟与全替身离线装配。

pub mod fakes;

#[allow(unused_imports)]
pub use self::fakes::{
    ConnectCounter, CountingAsrDelivery, CountingAudit, CountingInsight, MemoryGapLog,
    MemorySwitchLog, MonotonicSystemClock, RecordingStatusSink, SentinelRepos,
};

use std::sync::Arc;

use meeting_copilot_application::capture::{
    CaptureChannelService, CapturePolicy, CountingDiagnostics, SessionContextView,
};
use meeting_copilot_domain::model::{ChannelPreference, Platform, SessionState};
use meeting_copilot_domain::vo::SessionId;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{ClockError, MonoMillis, WallClock};

pub struct ManualClock {
    mono_ms: Mutex<u64>,
    wall_base: WallClock,
    wall_offset_ms: Mutex<u64>,
    fail_mono: AtomicBool,
}

impl ManualClock {
    pub fn new(wall_base: WallClock) -> Self {
        Self {
            mono_ms: Mutex::new(0),
            wall_base,
            wall_offset_ms: Mutex::new(0),
            fail_mono: AtomicBool::new(false),
        }
    }

    pub fn advance_by(&self, delta_ms: u64) {
        let mut mono = self
            .mono_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *mono = mono.saturating_add(delta_ms);
        let mut wall = self
            .wall_offset_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *wall = wall.saturating_add(delta_ms);
    }

    pub fn fail_mono(&self) {
        self.fail_mono.store(true, Ordering::SeqCst);
    }

    pub fn recover_mono(&self) {
        self.fail_mono.store(false, Ordering::SeqCst);
    }
}

impl CaptureClock for ManualClock {
    fn mono_now(&self) -> Result<MonoMillis, ClockError> {
        if self.fail_mono.load(Ordering::SeqCst) {
            return Err(ClockError::Overflow);
        }
        let mono = *self
            .mono_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        Ok(MonoMillis::from_millis(mono))
    }

    fn wall_now(&self) -> Result<WallClock, ClockError> {
        let offset = *self
            .wall_offset_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let offset = i64::try_from(offset).map_err(|_| ClockError::Overflow)?;
        Ok(WallClock::from_unix_millis(
            self.wall_base.as_unix_millis().saturating_add(offset),
        ))
    }
}
/// 单一全替身装配：≥4 个测试文件复用（AC-B2-OC-E06）。
pub struct OfflineHarness {
    pub service: CaptureChannelService,
    pub clock: Arc<ManualClock>,
    pub gaps: Arc<MemoryGapLog>,
    pub switches: Arc<MemorySwitchLog>,
    pub asr: Arc<CountingAsrDelivery>,
    pub insight: Arc<CountingInsight>,
    pub diagnostics: Arc<CountingDiagnostics>,
    pub audit: Arc<CountingAudit>,
    pub connections: Arc<ConnectCounter>,
    pub sentinel: Arc<SentinelRepos>,
    pub status_sink: Arc<RecordingStatusSink>,
}

pub fn build_offline_capture(policy: CapturePolicy) -> OfflineHarness {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let gaps = Arc::new(MemoryGapLog::default());
    let switches = Arc::new(MemorySwitchLog::default());
    let asr = Arc::new(CountingAsrDelivery::default());
    let insight = Arc::new(CountingInsight::default());
    let diagnostics = Arc::new(CountingDiagnostics::new());
    let audit = Arc::new(CountingAudit::default());
    let connections = Arc::new(ConnectCounter::default());
    let sentinel = Arc::new(SentinelRepos::default());
    let status_sink = Arc::new(RecordingStatusSink::default());
    let service = CaptureChannelService::with_offline_ports(
        clock.clone(),
        switches.clone(),
        gaps.clone(),
        status_sink.clone(),
        asr.clone(),
        insight.clone(),
        diagnostics.clone(),
        policy,
    );
    OfflineHarness {
        service,
        clock,
        gaps,
        switches,
        asr,
        insight,
        diagnostics,
        audit,
        connections,
        sentinel,
        status_sink,
    }
}

pub fn offline_view(session: &str) -> SessionContextView {
    SessionContextView {
        session: SessionId::new(session).expect("session"),
        platform: Platform::TencentMeeting,
        channel_preference: ChannelPreference::SubtitleFirst,
        session_state: SessionState::Listening,
    }
}
