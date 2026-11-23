//! WASAPI 共享模式环回采集：读取系统输出混音并转为单声道 f32 分片。
//!
//! 只读取默认输出设备的环回流，不注入、不录制麦克风；无新数据时返回 `None`。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::Platform;
use meeting_copilot_domain::port::capture::{
    AudioChunk, AudioSource, SourceHealth, SubtitleDescriptor,
};
use meeting_copilot_domain::vo::AudioClock;
use windows::core::GUID;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED,
};

const BUFFER_DURATION: i64 = 1_000_000;
const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const BUFFER_FLAG_SILENT: u32 = 0x2;
const SUBTYPE_PCM: GUID = GUID::from_u128(0x0000_0001_0000_0010_8000_00aa_0038_9b71);
const SUBTYPE_IEEE_FLOAT: GUID = GUID::from_u128(0x0000_0003_0000_0010_8000_00aa_0038_9b71);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SampleKind {
    Float32,
    Int16,
}

#[derive(Debug, Clone, Copy)]
struct FormatInfo {
    channels: usize,
    sample_rate: u32,
    block_align: usize,
    kind: SampleKind,
}

impl FormatInfo {
    unsafe fn from_mix(format: *const WAVEFORMATEX) -> Self {
        let base = *format;
        let mut kind = SampleKind::Int16;
        match base.wFormatTag {
            WAVE_FORMAT_IEEE_FLOAT => kind = SampleKind::Float32,
            WAVE_FORMAT_PCM => kind = SampleKind::Int16,
            WAVE_FORMAT_EXTENSIBLE => {
                let ext = format as *const WAVEFORMATEXTENSIBLE;
                let subtype = (*ext).SubFormat;
                if subtype == SUBTYPE_IEEE_FLOAT {
                    kind = SampleKind::Float32;
                } else if subtype == SUBTYPE_PCM {
                    kind = SampleKind::Int16;
                }
            }
            _ => {}
        }
        Self {
            channels: usize::from(base.nChannels).max(1),
            sample_rate: base.nSamplesPerSec,
            block_align: usize::from(base.nBlockAlign).max(1),
            kind,
        }
    }
}

fn decode_mono(bytes: &[u8], format: &FormatInfo, out: &mut Vec<f32>) {
    let channels = format.channels.max(1);
    match format.kind {
        SampleKind::Float32 => {
            let (chunks, _) = bytes.as_chunks::<4>();
            let mut sum = 0.0f32;
            let mut index = 0usize;
            for chunk in chunks {
                let value = f32::from_le_bytes(*chunk);
                sum += value;
                index += 1;
                if index.is_multiple_of(channels) {
                    out.push((sum / channels as f32).clamp(-1.0, 1.0));
                    sum = 0.0;
                }
            }
        }
        SampleKind::Int16 => {
            let (chunks, _) = bytes.as_chunks::<2>();
            let mut sum = 0f32;
            let mut index = 0usize;
            for chunk in chunks {
                let value = i16::from_le_bytes(*chunk);
                sum += f32::from(value) / 32_768.0;
                index += 1;
                if index.is_multiple_of(channels) {
                    out.push((sum / channels as f32).clamp(-1.0, 1.0));
                    sum = 0.0;
                }
            }
        }
    }
}

struct MonoFrame {
    sample_rate: u32,
    elapsed_ms: u64,
    samples: Vec<f32>,
}

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
            // 线程已按其它套间模型初始化 COM；接口仍可用，不能重复 CoUninitialize。
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

struct Runtime {
    // 字段按声明顺序析构：先释放采集/客户端接口，最后由 ComGuard 配对 CoUninitialize。
    capture: IAudioCaptureClient,
    client: IAudioClient,
    format: FormatInfo,
    frames: u64,
    _com: ComGuard,
}

// SAFETY: 所有 COM 对象只在 `WasapiLoopbackSource::runtime` 的 Mutex 保护下串行使用，
// 不跨线程并发访问接口。
unsafe impl Send for Runtime {}

impl Runtime {
    unsafe fn open() -> Result<Self, String> {
        let com = ComGuard::init()?;
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(
            &MMDeviceEnumerator,
            None::<&windows::core::IUnknown>,
            CLSCTX_ALL,
        )
        .map_err(|error| format!("创建设备枚举器失败：{error}"))?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|error| format!("获取默认输出设备失败：{error}"))?;
        let client: IAudioClient = device
            .Activate::<IAudioClient>(CLSCTX_ALL, None)
            .map_err(|error| format!("激活音频客户端失败：{error}"))?;
        let format_ptr = client
            .GetMixFormat()
            .map_err(|error| format!("读取混音格式失败：{error}"))?;
        let format = FormatInfo::from_mix(format_ptr);
        let initialized = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            BUFFER_DURATION,
            0,
            format_ptr,
            None,
        );
        CoTaskMemFree(Some(format_ptr.cast()));
        initialized.map_err(|error| format!("初始化环回流失败：{error}"))?;
        let capture: IAudioCaptureClient = client
            .GetService::<IAudioCaptureClient>()
            .map_err(|error| format!("获取采集客户端失败：{error}"))?;
        client
            .Start()
            .map_err(|error| format!("启动环回流失败：{error}"))?;
        Ok(Self {
            capture,
            client,
            format,
            frames: 0,
            _com: com,
        })
    }

    unsafe fn next_frame(&mut self) -> Result<Option<MonoFrame>, String> {
        let packet = self
            .capture
            .GetNextPacketSize()
            .map_err(|error| format!("读取包大小失败：{error}"))?;
        if packet == 0 {
            return Ok(None);
        }
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut frames = 0u32;
        let mut flags = 0u32;
        self.capture
            .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
            .map_err(|error| format!("读取缓冲失败：{error}"))?;
        let mut samples = Vec::new();
        let silent = flags & BUFFER_FLAG_SILENT != 0;
        if !silent && !data.is_null() && frames > 0 {
            let byte_len = frames as usize * self.format.block_align;
            let bytes = std::slice::from_raw_parts(data, byte_len);
            decode_mono(bytes, &self.format, &mut samples);
        } else {
            samples.resize(frames as usize, 0.0);
        }
        self.capture
            .ReleaseBuffer(frames)
            .map_err(|error| format!("释放缓冲失败：{error}"))?;
        self.frames = self.frames.saturating_add(u64::from(frames));
        let elapsed_ms = if self.format.sample_rate == 0 {
            0
        } else {
            self.frames.saturating_mul(1_000) / u64::from(self.format.sample_rate)
        };
        Ok(Some(MonoFrame {
            sample_rate: self.format.sample_rate,
            elapsed_ms,
            samples,
        }))
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // SAFETY: 停止已启动的环回流；失败不影响释放流程。
        unsafe {
            let _ = self.client.Stop();
        }
    }
}

/// WASAPI 共享模式环回采集源。
pub struct WasapiLoopbackSource {
    descriptor: SubtitleDescriptor,
    runtime: Mutex<Option<Runtime>>,
    sequence: AtomicU64,
    health: Mutex<SourceHealth>,
}

impl WasapiLoopbackSource {
    pub fn new(platform: Platform) -> Self {
        Self {
            descriptor: SubtitleDescriptor {
                platform,
                version: "wasapi-loopback".to_owned(),
                channel: "system_audio".to_owned(),
            },
            runtime: Mutex::new(None),
            sequence: AtomicU64::new(0),
            health: Mutex::new(SourceHealth::Unavailable("未打开".to_owned())),
        }
    }

    fn set_health(&self, next: SourceHealth) {
        if let Ok(mut health) = self.health.lock() {
            *health = next;
        }
    }

    fn lock_error() -> CopilotError {
        CopilotError::AudioCaptureFailed {
            detail: "采集锁不可用".to_owned(),
        }
    }
}

impl AudioSource for WasapiLoopbackSource {
    fn descriptor(&self) -> &SubtitleDescriptor {
        &self.descriptor
    }

    fn open(&self, _device: Option<&str>) -> Result<(), CopilotError> {
        let mut guard = self.runtime.lock().map_err(|_| Self::lock_error())?;
        if guard.is_some() {
            return Ok(());
        }
        // SAFETY: 初始化 COM 与环回流；Runtime 在 Mutex 内独占持有。
        match unsafe { Runtime::open() } {
            Ok(runtime) => {
                *guard = Some(runtime);
                self.set_health(SourceHealth::Ready);
                Ok(())
            }
            Err(detail) => {
                self.set_health(SourceHealth::Unavailable(detail.clone()));
                Err(CopilotError::AudioDeviceUnavailable { detail })
            }
        }
    }

    fn next_chunk(&self) -> Result<Option<AudioChunk>, CopilotError> {
        let mut guard = self.runtime.lock().map_err(|_| Self::lock_error())?;
        let Some(runtime) = guard.as_mut() else {
            return Err(CopilotError::AudioDeviceUnavailable {
                detail: "尚未打开采集设备".to_owned(),
            });
        };
        // SAFETY: 运行时对象由 Mutex 独占；数据指针在 ReleaseBuffer 前有效。
        let frame = match unsafe { runtime.next_frame() } {
            Ok(frame) => frame,
            Err(detail) => {
                self.set_health(SourceHealth::Degraded(detail.clone()));
                return Err(CopilotError::AudioCaptureFailed { detail });
            }
        };
        let Some(frame) = frame else {
            return Ok(None);
        };
        if frame.samples.is_empty() {
            return Ok(None);
        }
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        Ok(Some(AudioChunk {
            sequence,
            clock: AudioClock::from_millis(frame.elapsed_ms),
            sample_rate: frame.sample_rate,
            samples: frame.samples,
        }))
    }

    fn health(&self) -> SourceHealth {
        self.health
            .lock()
            .map(|health| health.clone())
            .unwrap_or_else(|_| SourceHealth::Unavailable("状态不可用".to_owned()))
    }
}

static SYSTEM_AUDIO: OnceLock<WasapiLoopbackSource> = OnceLock::new();

/// 系统音频通道只读状态：首次调用时尝试打开默认输出设备。
pub fn system_audio_status() -> &'static str {
    let source = SYSTEM_AUDIO.get_or_init(|| {
        let source = WasapiLoopbackSource::new(Platform::TencentMeeting);
        let _ = source.open(None);
        source
    });
    match source.health() {
        SourceHealth::Ready => "ready",
        SourceHealth::Degraded(_) => "degraded",
        SourceHealth::Unavailable(_) => "off",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(kind: SampleKind, channels: usize) -> FormatInfo {
        FormatInfo {
            channels,
            sample_rate: 48_000,
            block_align: channels * if kind == SampleKind::Float32 { 4 } else { 2 },
            kind,
        }
    }

    #[test]
    fn stereo_float_is_mixed_to_mono() {
        let mut bytes = Vec::new();
        for sample in [0.5f32, -0.5, 1.0, 0.0] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let mut out = Vec::new();
        decode_mono(&bytes, &format(SampleKind::Float32, 2), &mut out);
        assert_eq!(out.len(), 2);
        assert!(out[0].abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn stereo_int16_is_mixed_and_normalized() {
        let mut bytes = Vec::new();
        for sample in [16_384i16, -16_384, 32_767, 0] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let mut out = Vec::new();
        decode_mono(&bytes, &format(SampleKind::Int16, 2), &mut out);
        assert_eq!(out.len(), 2);
        assert!(out[0].abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn mono_passthrough_keeps_samples() {
        let mut bytes = Vec::new();
        for sample in [0.25f32, -0.25] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let mut out = Vec::new();
        decode_mono(&bytes, &format(SampleKind::Float32, 1), &mut out);
        assert_eq!(out, vec![0.25, -0.25]);
    }

    #[test]
    #[ignore = "需要本机默认输出设备；CI 无音频设备"]
    fn loopback_opens_and_captures_when_audio_is_active() {
        let source = WasapiLoopbackSource::new(Platform::TencentMeeting);
        source.open(None).expect("打开默认输出设备");
        assert_eq!(source.health(), SourceHealth::Ready);
        for _ in 0..200 {
            if let Some(chunk) = source.next_chunk().expect("读取分片") {
                assert!(chunk.sample_rate > 0);
                assert!(chunk.sequence > 0);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // 设备无音频活动时 WASAPI 不产生环回包，属平台行为：open/health 已证明链路可用。
    }
}
