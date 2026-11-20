use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::CopilotError;
use crate::model::enums::{Platform, ReadingMode};
use crate::model::session::TextRange;
use crate::model::utterance::SpeakerRef;
use crate::vo::{AudioClock, Confidence, UtteranceText};

/// 采集通道健康状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceHealth {
    Ready,
    Degraded(String),
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleCapability {
    pub available: bool,
    pub reason: String,
}

/// 字幕通道的一次事件。字幕是首选通道（design 2.5.3）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitleEvent {
    pub span: TextRange,
    pub text: UtteranceText,
    pub speaker: SpeakerRef,
    pub confidence: Confidence,
    pub platform: Platform,
    pub reading_mode: ReadingMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleDescriptor {
    pub platform: Platform,
    pub version: String,
    pub channel: String,
}

/// 订阅令牌。`release` 之后所有包装回调必须静默丢弃事件。
#[derive(Debug, Clone)]
pub struct Subscription {
    released: Arc<AtomicBool>,
}

impl Subscription {
    pub fn new() -> Self {
        Self {
            released: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_active(&self) -> bool {
        !self.released.load(Ordering::Acquire)
    }

    pub fn release(&self) {
        self.released.store(true, Ordering::Release);
    }

    /// 把任意回调包装成"释放后自动失效"的安全回调。
    pub fn wrap<F>(&self, callback: F) -> impl Fn(SubtitleEvent) + Send + Sync + 'static
    where
        F: Fn(SubtitleEvent) + Send + Sync + 'static,
    {
        let released = Arc::clone(&self.released);
        move |event| {
            if !released.load(Ordering::Acquire) {
                callback(event);
            }
        }
    }
}

impl Default for Subscription {
    fn default() -> Self {
        Self::new()
    }
}

pub trait SubtitleSource: Send + Sync {
    fn descriptor(&self) -> &SubtitleDescriptor;

    fn probe(&self) -> Result<SubtitleCapability, CopilotError>;

    fn subscribe(
        &self,
        sink: Box<dyn Fn(SubtitleEvent) + Send + Sync>,
    ) -> Result<Subscription, CopilotError>;

    fn health(&self) -> SourceHealth;

    fn release(&self) -> Result<(), CopilotError>;
}

/// 一段系统音频。样本始终按单声道 f32 归一化到 `[-1, 1]`。
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    pub sequence: u64,
    pub clock: AudioClock,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

impl AudioChunk {
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        u64::try_from(self.samples.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(1_000)
            / u64::from(self.sample_rate)
    }
}

pub trait AudioSource: Send + Sync {
    fn descriptor(&self) -> &SubtitleDescriptor;

    fn open(&self, device: Option<&str>) -> Result<(), CopilotError>;

    fn next_chunk(&self) -> Result<Option<AudioChunk>, CopilotError>;

    fn health(&self) -> SourceHealth;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> SubtitleEvent {
        SubtitleEvent {
            span: TextRange::new(0, 1_000).expect("span"),
            text: UtteranceText::new("测试字幕").expect("text"),
            speaker: SpeakerRef::unknown(),
            confidence: Confidence::new(0.9).expect("conf"),
            platform: Platform::Zoom,
            reading_mode: ReadingMode::Subtitle,
        }
    }

    #[test]
    fn subscription_drops_events_after_release() {
        let subscription = Subscription::new();
        let hits = Arc::new(AtomicBool::new(false));

        let hits_clone = Arc::clone(&hits);
        let sink = subscription.wrap(move |_event| {
            hits_clone.store(true, Ordering::Release);
        });

        sink(event());
        assert!(hits.load(Ordering::Acquire));

        hits.store(false, Ordering::Release);
        subscription.release();
        assert!(!subscription.is_active());
        sink(event());
        assert!(!hits.load(Ordering::Acquire), "释放后不得再回调");
    }

    #[test]
    fn audio_duration_from_samples() {
        let chunk = AudioChunk {
            sequence: 1,
            clock: AudioClock::ZERO,
            sample_rate: 16_000,
            samples: vec![0.0; 8_000],
        };
        assert_eq!(chunk.duration_ms(), 500);
    }
}