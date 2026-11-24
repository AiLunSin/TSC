//! UI Automation 字幕通道：轮询会议窗口已呈现文本，差量生成字幕事件。
//!
//! 只读取窗口内 UIA `Name` 文本，不截屏、不注入；找不到窗口时降级而非报错。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::{Platform, ReadingMode};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::port::capture::{
    SourceHealth, Subscription, SubtitleCapability, SubtitleDescriptor, SubtitleEvent,
    SubtitleSource,
};
use meeting_copilot_domain::vo::{Confidence, UtteranceText};

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const POLL_SLICE: Duration = Duration::from_millis(50);
const STATUS_TTL: Duration = Duration::from_millis(1_500);
const UIA_CONFIDENCE: f64 = 0.8;
const MS_PER_CHAR: u64 = 120;
const MIN_SPAN_MS: u64 = 500;

type EventSink = Arc<dyn Fn(SubtitleEvent) + Send + Sync>;

enum ScanOutcome {
    Ready {
        display_name: &'static str,
        text: String,
    },
    NoWindow,
    NoText {
        elements: usize,
    },
    Failed(String),
}

fn platform_for(display_name: &str) -> Platform {
    match display_name {
        "飞书" => Platform::Feishu,
        _ => Platform::TencentMeeting,
    }
}

fn ocr_fallback_enabled() -> bool {
    std::env::var("OCR_FALLBACK")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    text.chars().take(limit).collect()
}

fn diff_increment(previous: &str, current: &str) -> Option<String> {
    let current = current.trim();
    if current.is_empty() {
        return None;
    }
    let previous = previous.trim();
    if previous.is_empty() {
        return Some(current.to_owned());
    }
    if current == previous || previous.starts_with(current) {
        return None;
    }
    if let Some(rest) = current.strip_prefix(previous) {
        let rest = rest.trim();
        return (!rest.is_empty()).then(|| rest.to_owned());
    }
    let shared = previous
        .char_indices()
        .zip(current.char_indices())
        .take_while(|((_, left), (_, right))| left == right)
        .count();
    let start = current
        .char_indices()
        .nth(shared)
        .map(|(index, _)| index)
        .unwrap_or(current.len());
    let increment = current[start..].trim();
    (!increment.is_empty()).then(|| increment.to_owned())
}

fn merge_names(names: &[String]) -> String {
    let mut merged = String::new();
    let mut seen = std::collections::HashSet::new();
    for name in names {
        let text = name.trim();
        if text.is_empty() || !seen.insert(text) {
            continue;
        }
        if !merged.is_empty() {
            merged.push('\n');
        }
        merged.push_str(text);
        if merged.chars().count() >= UtteranceText::LIMIT {
            break;
        }
    }
    truncate_chars(&merged, UtteranceText::LIMIT)
}

fn build_event(text: String, platform: Platform, start_ms: u64) -> Option<SubtitleEvent> {
    let text = UtteranceText::new(truncate_chars(&text, UtteranceText::LIMIT)).ok()?;
    let char_count = u64::try_from(text.char_count()).unwrap_or(u64::MAX);
    let duration = char_count.saturating_mul(MS_PER_CHAR).max(MIN_SPAN_MS);
    let span = TextRange::new(start_ms, start_ms.checked_add(duration)?)?;
    Some(SubtitleEvent {
        span,
        text,
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(UIA_CONFIDENCE).ok()?,
        platform,
        reading_mode: ReadingMode::Subtitle,
    })
}

struct Inner {
    sink: Option<EventSink>,
    subscription: Option<Subscription>,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    health: SourceHealth,
    last_text: String,
    started: Instant,
}

impl Inner {
    fn fresh() -> Self {
        Self {
            sink: None,
            subscription: None,
            running: Arc::new(AtomicBool::new(false)),
            worker: None,
            health: SourceHealth::Unavailable("尚未启动".to_owned()),
            last_text: String::new(),
            started: Instant::now(),
        }
    }
}

pub struct UiaSubtitleSource {
    descriptor: SubtitleDescriptor,
    inner: Arc<Mutex<Inner>>,
}

impl UiaSubtitleSource {
    pub fn new(platform: Platform) -> Self {
        Self {
            descriptor: SubtitleDescriptor {
                platform,
                version: "uia-1".to_owned(),
                channel: "uia_name_poll".to_owned(),
            },
            inner: Arc::new(Mutex::new(Inner::fresh())),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    pub fn is_running(&self) -> bool {
        self.lock().running.load(Ordering::Acquire)
    }

    pub fn stop(&self) {
        let (worker, subscription) = {
            let mut inner = self.lock();
            inner.running.store(false, Ordering::Release);
            inner.sink = None;
            (inner.worker.take(), inner.subscription.take())
        };
        if let Some(subscription) = subscription {
            subscription.release();
        }
        if let Some(worker) = worker {
            let _ = worker.join();
        }
        self.lock().health = SourceHealth::Unavailable("已停止".to_owned());
    }
}

impl SubtitleSource for UiaSubtitleSource {
    fn descriptor(&self) -> &SubtitleDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> Result<SubtitleCapability, CopilotError> {
        Ok(probe_capability())
    }

    fn subscribe(
        &self,
        sink: Box<dyn Fn(SubtitleEvent) + Send + Sync>,
    ) -> Result<Subscription, CopilotError> {
        self.stop();
        let sink: EventSink = Arc::from(sink);
        let running = Arc::new(AtomicBool::new(true));
        let subscription = Subscription::new();
        {
            let mut inner = self.lock();
            inner.sink = Some(Arc::clone(&sink));
            inner.subscription = Some(subscription.clone());
            inner.running = Arc::clone(&running);
            inner.last_text.clear();
            inner.started = Instant::now();
            inner.health = SourceHealth::Degraded("等待窗口文本".to_owned());
        }
        let inner = Arc::clone(&self.inner);
        let worker = std::thread::Builder::new()
            .name("uia-subtitle-poll".to_owned())
            .spawn(move || poll_loop(inner, running, sink))
            .map_err(|error| CopilotError::AsrFailed {
                detail: format!("启动字幕轮询失败：{error}"),
            })?;
        self.lock().worker = Some(worker);
        Ok(subscription)
    }

    fn health(&self) -> SourceHealth {
        self.lock().health.clone()
    }

    fn release(&self) -> Result<(), CopilotError> {
        self.stop();
        Ok(())
    }
}

impl Drop for UiaSubtitleSource {
    fn drop(&mut self) {
        self.stop();
    }
}

fn poll_loop(inner: Arc<Mutex<Inner>>, running: Arc<AtomicBool>, sink: EventSink) {
    while running.load(Ordering::Acquire) {
        let event = {
            let mut guard = inner.lock().unwrap_or_else(|poison| poison.into_inner());
            let outcome = imp::scan();
            let mut event = None;
            match outcome {
                ScanOutcome::Ready { display_name, text } => {
                    guard.health = SourceHealth::Ready;
                    let increment = diff_increment(&guard.last_text, &text);
                    guard.last_text = text;
                    if let Some(increment) = increment {
                        let elapsed =
                            u64::try_from(guard.started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        event = build_event(increment, platform_for(display_name), elapsed);
                    }
                }
                ScanOutcome::NoText { elements } => {
                    guard.health = SourceHealth::Degraded(format!(
                        "窗口内暂无可读字幕元素（已遍历 {elements} 个）"
                    ));
                }
                ScanOutcome::NoWindow => {
                    guard.health = SourceHealth::Unavailable("未找到会议窗口".to_owned());
                    guard.last_text.clear();
                }
                ScanOutcome::Failed(detail) => {
                    guard.health = SourceHealth::Degraded(detail);
                }
            }
            event
        };
        if let Some(event) = event {
            sink(event);
        }
        let mut waited = Duration::ZERO;
        while waited < POLL_INTERVAL {
            if !running.load(Ordering::Acquire) {
                return;
            }
            std::thread::sleep(POLL_SLICE);
            waited += POLL_SLICE;
        }
    }
}

/// 字幕通道只读状态：1.5 秒内复用上次探测结果，避免前端轮询重复遍历窗口。
pub fn subtitle_status() -> &'static str {
    static CACHE: OnceLock<Mutex<Option<(Instant, &'static str)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    {
        let guard = cache.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some((at, status)) = *guard {
            if at.elapsed() < STATUS_TTL {
                return status;
            }
        }
    }
    let status = match imp::scan() {
        ScanOutcome::Ready { .. } => "ready",
        ScanOutcome::NoText { .. } => "degraded",
        ScanOutcome::NoWindow | ScanOutcome::Failed(_) => "off",
    };
    let mut guard = cache.lock().unwrap_or_else(|poison| poison.into_inner());
    *guard = Some((Instant::now(), status));
    status
}

fn probe_capability() -> SubtitleCapability {
    match imp::scan() {
        ScanOutcome::Ready { text, .. } => SubtitleCapability {
            available: true,
            reason: format!("可读文本 {} 字", text.chars().count()),
        },
        ScanOutcome::NoText { elements } => SubtitleCapability {
            available: false,
            reason: format!("窗口内暂无可读字幕元素（已遍历 {elements} 个）"),
        },
        ScanOutcome::NoWindow => SubtitleCapability {
            available: false,
            reason: "未找到会议窗口".to_owned(),
        },
        ScanOutcome::Failed(detail) => SubtitleCapability {
            available: false,
            reason: detail,
        },
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
        TreeScope_Descendants, UIA_TextPatternId,
    };

    use super::{merge_names, ocr_fallback_enabled, ScanOutcome};

    const MAX_ELEMENTS: i32 = 600;
    const RPC_E_CHANGED_MODE: i32 = 0x8001_0106u32 as i32;

    struct ComGuard {
        owned: bool,
    }

    impl ComGuard {
        fn init() -> Result<Self, String> {
            // SAFETY: 在当前线程初始化 COM；只有本结构体真正初始化成功时才在 Drop 配对释放。
            let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if result.is_ok() {
                return Ok(Self { owned: true });
            }
            if result.0 == RPC_E_CHANGED_MODE {
                return Ok(Self { owned: false });
            }
            Err(format!("COM 初始化失败：{result:?}"))
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.owned {
                // SAFETY: 与 ComGuard::init 的 CoInitializeEx 成对。
                unsafe { CoUninitialize() };
            }
        }
    }

    fn create_automation() -> Result<IUIAutomation, String> {
        // SAFETY: CLSID 来自 windows crate 常量，接口类型与 CLSID 匹配。
        unsafe {
            CoCreateInstance(
                &CUIAutomation,
                None::<&windows::core::IUnknown>,
                CLSCTX_INPROC_SERVER,
            )
        }
        .map_err(|error| format!("创建 UI Automation 失败：{error}"))
    }

    struct WindowText {
        elements: usize,
        names: Vec<String>,
    }

    fn read_window_text(automation: &IUIAutomation, handle: isize) -> Result<WindowText, String> {
        let hwnd = HWND(handle as *mut c_void);
        // SAFETY: hwnd 来自 EnumWindows；automation 在本线程创建，调用期间保持存活。
        let root = unsafe { automation.ElementFromHandle(hwnd) }
            .map_err(|error| format!("定位窗口元素失败：{error}"))?;
        // SAFETY: 同上，接口调用与对象生命周期一致。
        let condition = unsafe { automation.CreateTrueCondition() }
            .map_err(|error| format!("创建遍历条件失败：{error}"))?;
        // SAFETY: 条件对象在调用期间存活。
        let collection = unsafe { root.FindAll(TreeScope_Descendants, &condition) }
            .map_err(|error| format!("遍历窗口元素失败：{error}"))?;
        // SAFETY: 元素数组接口来自上一步。
        let length =
            unsafe { collection.Length() }.map_err(|error| format!("读取元素数量失败：{error}"))?;
        let mut names = Vec::new();
        let mut elements = 0usize;
        for index in 0..length.min(MAX_ELEMENTS) {
            // SAFETY: index 在 [0, min(length, MAX_ELEMENTS)) 范围内。
            let Ok(element) = (unsafe { collection.GetElement(index) }) else {
                continue;
            };
            elements += 1;
            // SAFETY: 元素接口来自上一步，只在当前作用域使用。
            let Ok(name) = (unsafe { element.CurrentName() }) else {
                continue;
            };
            let text = name.to_string();
            if !text.trim().is_empty() {
                names.push(text);
            }
        }
        if names.is_empty() {
            if let Some(text) = read_text_pattern(&root) {
                names.push(text);
            }
        }
        if names.is_empty() && ocr_fallback_enabled() {
            if let Ok(lines) = crate::ocr::recognize_window(handle) {
                names.extend(lines);
            }
        }
        Ok(WindowText { elements, names })
    }

    fn read_text_pattern(root: &IUIAutomationElement) -> Option<String> {
        // SAFETY: 窗口元素与 TextPattern 在调用期间保持存活。
        unsafe {
            let pattern = root
                .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                .ok()?;
            let range = pattern.DocumentRange().ok()?;
            let text = range.GetText(-1).ok()?.to_string();
            (!text.trim().is_empty()).then_some(text)
        }
    }

    pub(super) fn scan() -> ScanOutcome {
        let Some(windows) = crate::meeting::find_meeting_windows() else {
            return ScanOutcome::NoWindow;
        };
        let _com = match ComGuard::init() {
            Ok(guard) => guard,
            Err(detail) => return ScanOutcome::Failed(detail),
        };
        let automation = match create_automation() {
            Ok(automation) => automation,
            Err(detail) => return ScanOutcome::Failed(detail),
        };
        let mut names = Vec::new();
        let mut elements = 0usize;
        let mut last_error = None;
        let mut succeeded = 0usize;
        let mut text_owner = None;
        for (display_name, handle) in windows {
            match read_window_text(&automation, handle) {
                Ok(mut found) => {
                    succeeded += 1;
                    elements += found.elements;
                    if !found.names.is_empty() && text_owner.is_none() {
                        text_owner = Some(display_name);
                    }
                    names.append(&mut found.names);
                }
                Err(detail) => last_error = Some(detail),
            }
        }
        if succeeded == 0 {
            return ScanOutcome::Failed(
                last_error.unwrap_or_else(|| "读取窗口文本失败".to_owned()),
            );
        }
        let text = merge_names(&names);
        if text.is_empty() {
            return ScanOutcome::NoText { elements };
        }
        ScanOutcome::Ready {
            display_name: text_owner.unwrap_or("腾讯会议"),
            text,
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::ScanOutcome;

    pub(super) fn scan() -> ScanOutcome {
        ScanOutcome::Failed("当前平台不支持 UI Automation 字幕".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_line_when_previous_empty() {
        assert_eq!(diff_increment("", "第一句"), Some("第一句".to_owned()));
    }

    #[test]
    fn only_new_tail_when_text_grows() {
        assert_eq!(
            diff_increment("第一句", "第一句第二句"),
            Some("第二句".to_owned())
        );
    }

    #[test]
    fn unchanged_and_shrinking_text_is_ignored() {
        assert_eq!(diff_increment("第一句", "第一句"), None);
        assert_eq!(diff_increment("第一句第二句", "第一句"), None);
        assert_eq!(diff_increment("第一句", "   "), None);
    }

    #[test]
    fn replaced_line_keeps_suffix_after_shared_prefix() {
        assert_eq!(
            diff_increment("会议开始", "会议结束"),
            Some("结束".to_owned())
        );
    }

    #[test]
    fn merge_dedupes_and_limits_length() {
        let names = vec![
            " 字幕甲 ".to_owned(),
            "字幕甲".to_owned(),
            "字幕乙".to_owned(),
        ];
        assert_eq!(merge_names(&names), "字幕甲\n字幕乙");
        let long = vec!["字".repeat(UtteranceText::LIMIT + 50)];
        assert_eq!(merge_names(&long).chars().count(), UtteranceText::LIMIT);
    }

    #[test]
    fn event_span_is_positive_and_monotonic() {
        let event =
            build_event("会议关键词".to_owned(), Platform::TencentMeeting, 1_000).expect("event");
        assert!(event.span.start_ms < event.span.end_ms);
        assert_eq!(event.span.start_ms, 1_000);
        assert_eq!(event.reading_mode, ReadingMode::Subtitle);
    }

    #[test]
    fn missing_platform_defaults_to_tencent_meeting() {
        assert_eq!(platform_for("腾讯会议"), Platform::TencentMeeting);
        assert_eq!(platform_for("飞书"), Platform::Feishu);
        assert_eq!(platform_for("微信"), Platform::TencentMeeting);
    }

    #[test]
    fn new_source_reports_unavailable_before_start() {
        let source = UiaSubtitleSource::new(Platform::TencentMeeting);
        assert!(!source.is_running());
        assert!(matches!(
            source.health(),
            SourceHealth::Unavailable(reason) if reason == "尚未启动"
        ));
        assert_eq!(source.descriptor().channel, "uia_name_poll");
    }

    #[test]
    fn stop_is_idempotent() {
        let source = UiaSubtitleSource::new(Platform::TencentMeeting);
        source.stop();
        source.stop();
        assert!(!source.is_running());
    }

    #[test]
    fn status_probe_reports_known_state_without_panicking() {
        let status = subtitle_status();
        assert!(
            matches!(status, "ready" | "degraded" | "off"),
            "unexpected subtitle status: {status}"
        );
    }

    #[test]
    fn capability_probe_returns_readable_reason() {
        let capability = probe_capability();
        assert!(!capability.reason.is_empty());
        if capability.available {
            assert!(capability.reason.contains("可读文本"));
        }
    }
}
