use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::{AbsoluteUrl, DedupeKey};
use meeting_copilot_domain::model::enums::{Platform, ProviderKind, ReadingMode, TrustTier};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::port::capture::{
    AudioChunk, AudioSource, SourceHealth, Subscription, SubtitleCapability, SubtitleDescriptor,
    SubtitleEvent, SubtitleSource,
};
use meeting_copilot_domain::port::governance::{Sanitizer, SensitiveHit};
use meeting_copilot_domain::port::search::{
    ProviderFailure, ProviderRequest, ProviderResponse, SearchHit, SearchProvider,
};
use meeting_copilot_domain::vo::{AudioClock, Confidence, ProviderId, TitleText, UtteranceText};

const SUPPORTED_SPI: u16 = 1;

fn descriptor(platform: Platform, channel: &str) -> SubtitleDescriptor {
    SubtitleDescriptor {
        platform,
        version: "fixture-1.0".to_owned(),
        channel: channel.to_owned(),
    }
}

fn sample_event() -> SubtitleEvent {
    SubtitleEvent {
        span: TextRange::new(0, 1_200).expect("span"),
        text: UtteranceText::new("我们把灰度范围收窄到两个站点").expect("text"),
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(0.92).expect("conf"),
        platform: Platform::TencentMeeting,
        reading_mode: ReadingMode::Subtitle,
    }
}

type Sink = Arc<dyn Fn(SubtitleEvent) + Send + Sync>;

struct MockSubtitleSource {
    descriptor: SubtitleDescriptor,
    sinks: Mutex<Vec<Sink>>,
}

impl MockSubtitleSource {
    fn new() -> Self {
        Self {
            descriptor: descriptor(Platform::TencentMeeting, "subtitle"),
            sinks: Mutex::new(Vec::new()),
        }
    }

    fn emit(&self, event: SubtitleEvent) {
        let sinks: Vec<Sink> = self
            .sinks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        for sink in sinks {
            sink(event.clone());
        }
    }
}

impl SubtitleSource for MockSubtitleSource {
    fn descriptor(&self) -> &SubtitleDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> Result<SubtitleCapability, CopilotError> {
        Ok(SubtitleCapability {
            available: true,
            reason: "mock".to_owned(),
        })
    }

    fn subscribe(
        &self,
        sink: Box<dyn Fn(SubtitleEvent) + Send + Sync>,
    ) -> Result<Subscription, CopilotError> {
        let subscription = Subscription::new();
        let guarded: Sink = Arc::new(subscription.wrap(sink));
        self.sinks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(guarded);
        Ok(subscription)
    }

    fn health(&self) -> SourceHealth {
        SourceHealth::Ready
    }

    fn release(&self) -> Result<(), CopilotError> {
        self.sinks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        Ok(())
    }
}

struct MockAudioSource {
    descriptor: SubtitleDescriptor,
    opened: AtomicBool,
    sequence: AtomicU32,
}

impl MockAudioSource {
    fn new() -> Self {
        Self {
            descriptor: descriptor(Platform::Zoom, "system-audio"),
            opened: AtomicBool::new(false),
            sequence: AtomicU32::new(0),
        }
    }
}

impl AudioSource for MockAudioSource {
    fn descriptor(&self) -> &SubtitleDescriptor {
        &self.descriptor
    }

    fn open(&self, device: Option<&str>) -> Result<(), CopilotError> {
        if device == Some("missing") {
            return Err(CopilotError::AudioDeviceUnavailable {
                detail: "device missing".to_owned(),
            });
        }
        self.opened.store(true, Ordering::Release);
        Ok(())
    }

    fn next_chunk(&self) -> Result<Option<AudioChunk>, CopilotError> {
        if !self.opened.load(Ordering::Acquire) {
            return Err(CopilotError::AudioCaptureFailed {
                detail: "not opened".to_owned(),
            });
        }
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel);
        Ok(Some(AudioChunk {
            sequence: u64::from(sequence),
            clock: AudioClock::from_millis(u64::from(sequence) * 100),
            sample_rate: 16_000,
            samples: vec![0.0; 1_600],
        }))
    }

    fn health(&self) -> SourceHealth {
        if self.opened.load(Ordering::Acquire) {
            SourceHealth::Ready
        } else {
            SourceHealth::Unavailable("未打开".to_owned())
        }
    }
}

struct MockSanitizer;

impl Sanitizer for MockSanitizer {
    fn sanitize_query(&self, raw: &str) -> Result<(String, u16), CopilotError> {
        let mut text = raw.to_owned();
        let mut count = 0_u16;
        for (needle, replacement) in [("张三", "[人名]"), ("13800138000", "[手机号]")] {
            if text.contains(needle) {
                text = text.replace(needle, replacement);
                count = count.saturating_add(1);
            }
        }
        Ok((text, count))
    }

    fn sanitize_output(&self, raw: &str) -> Result<String, CopilotError> {
        Ok(self.sanitize_query(raw)?.0)
    }

    fn scan(&self, raw: &str) -> Vec<SensitiveHit> {
        let mut hits = Vec::new();
        if let Some(start) = raw.find("张三") {
            hits.push(SensitiveHit {
                kind: meeting_copilot_domain::port::governance::SensitiveKind::Person,
                start,
                len: 6,
            });
        }
        hits
    }
}

struct MockSearchProvider {
    id: ProviderId,
    spi_version: u16,
}

impl MockSearchProvider {
    fn new(spi_version: u16) -> Self {
        Self {
            id: ProviderId::new("mock-provider").expect("id"),
            spi_version,
        }
    }
}

impl SearchProvider for MockSearchProvider {
    fn profile_id(&self) -> &ProviderId {
        &self.id
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Custom
    }

    fn health(&self) -> meeting_copilot_domain::model::enums::ProviderHealth {
        meeting_copilot_domain::model::enums::ProviderHealth::Healthy
    }

    fn search(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderFailure> {
        if self.spi_version != SUPPORTED_SPI {
            return Err(ProviderFailure::MalformedResponse);
        }
        let hit = SearchHit {
            title: TitleText::new("模型上下文窗口说明").expect("title"),
            url: AbsoluteUrl::new("https://docs.example.com/context-window").expect("url"),
            domain: "docs.example.com".to_owned(),
            snippet: Some("上下文窗口用于衡量单次可处理的 token 数量".to_owned()),
            published_at: None,
            trust: TrustTier::Official,
        };
        let mut hits = vec![hit];
        hits.truncate(usize::from(request.limit));
        Ok(ProviderResponse {
            provider: self.id.clone(),
            hits,
            elapsed_ms: 120,
        })
    }
}

fn assert_subtitle_contract(source: &dyn SubtitleSource) {
    let capability = source.probe().expect("probe 不应失败");
    assert!(capability.available, "mock 字幕通道应可用");
    assert_eq!(source.health(), SourceHealth::Ready);

    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let received = Arc::clone(&received);
        move |event: SubtitleEvent| {
            received
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(event);
        }
    };
    let subscription = source.subscribe(Box::new(sink)).expect("订阅");
    assert!(subscription.is_active());
    subscription.release();
    assert!(!subscription.is_active(), "release 后必须失效");
}

fn assert_audio_contract(source: &dyn AudioSource) {
    assert!(source.open(None).is_ok());
    let chunk = source.next_chunk().expect("读取").expect("有数据");
    assert!(chunk.sample_rate > 0);
    assert_eq!(chunk.duration_ms(), 100);
    assert_eq!(source.health(), SourceHealth::Ready);

    let missing = MockAudioSource::new();
    assert!(matches!(
        missing.open(Some("missing")),
        Err(CopilotError::AudioDeviceUnavailable { .. })
    ));
}

fn assert_sanitizer_contract(sanitizer: &dyn Sanitizer) {
    let (text, count) = sanitizer
        .sanitize_query("张三 的手机号是 13800138000")
        .expect("脱敏");
    assert!(!text.contains("张三"));
    assert!(!text.contains("13800138000"));
    assert_eq!(count, 2);

    let query = sanitizer
        .issue_query("张三 的模型上下文窗口")
        .expect("产出查询");
    assert!(query.redaction_count() >= 1);
    assert!(!query.as_str().contains("张三"));

    assert!(sanitizer.scan("张三").iter().any(|hit| {
        hit.kind == meeting_copilot_domain::port::governance::SensitiveKind::Person
    }));
}

fn assert_search_contract(provider: &dyn SearchProvider, sanitizer: &dyn Sanitizer) {
    let query = sanitizer.issue_query("模型上下文窗口").expect("查询");
    let request = ProviderRequest::new(
        query,
        3,
        meeting_copilot_domain::model::provider::ProviderTimeout::from_millis(3_000).expect("超时"),
        vec![
            meeting_copilot_domain::model::provider::DomainPattern::new("*.internal.example.com")
                .expect("pattern"),
        ],
    )
    .expect("请求");

    let response = provider.search(&request).expect("检索");
    assert_eq!(response.provider, *provider.profile_id());
    assert!(!response.hits.is_empty());
    for hit in &response.hits {
        assert!(!request.is_domain_blocked(hit.url.domain().unwrap_or_default()));
    }
}

#[test]
fn mock_ports_satisfy_contracts() {
    let subtitle = MockSubtitleSource::new();
    assert_subtitle_contract(&subtitle);

    let audio = MockAudioSource::new();
    assert_audio_contract(&audio);

    let sanitizer = MockSanitizer;
    assert_sanitizer_contract(&sanitizer);

    let provider = MockSearchProvider::new(SUPPORTED_SPI);
    assert_search_contract(&provider, &sanitizer);
}

#[test]
fn incompatible_spi_version_is_rejected() {
    let provider = MockSearchProvider::new(2);
    let query = MockSanitizer.issue_query("模型上下文窗口").expect("查询");
    let request = ProviderRequest::new(
        query,
        3,
        meeting_copilot_domain::model::provider::ProviderTimeout::from_millis(3_000).expect("超时"),
        Vec::new(),
    )
    .expect("请求");
    assert!(matches!(
        provider.search(&request),
        Err(ProviderFailure::MalformedResponse)
    ));
}

#[test]
fn released_subscription_stops_delivery() {
    let source = MockSubtitleSource::new();
    let received = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&received);
    let subscription = source
        .subscribe(Box::new(move |_event| {
            counter.fetch_add(1, Ordering::AcqRel);
        }))
        .expect("订阅");

    source.emit(sample_event());
    assert_eq!(received.load(Ordering::Acquire), 1);

    subscription.release();
    source.emit(sample_event());
    assert_eq!(received.load(Ordering::Acquire), 1, "释放后不得再计数");
}

#[test]
fn dedupe_key_rejects_bad_shape() {
    assert!(DedupeKey::new("abc").is_err());
    assert!(DedupeKey::new("a".repeat(32)).is_ok());
}
