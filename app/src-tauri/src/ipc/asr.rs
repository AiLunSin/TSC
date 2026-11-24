//! 实时转写管线：WASAPI 环回 → 16 kHz PCM → vosk 流式 → 关键词卡片。
//!
//! Python 子进程只做识别；采集、重采样与出卡都在进程内完成。

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use meeting_copilot_domain::model::enums::Platform;
use meeting_copilot_domain::port::capture::{
    AudioSource, SourceHealth, SubtitleEvent, SubtitleSource,
};
use meeting_copilot_platform_win::subtitle::UiaSubtitleSource;
use meeting_copilot_platform_win::wasapi::WasapiLoopbackSource;

use crate::ipc::card_store::CardStore;
use crate::ipc::dto::IpcErrorDto;

const DEV_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const MODEL_DIR_NAME: &str = "vosk-model-small-cn-0.22";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// ASR 资源定位：发布包优先使用 `resource_dir/asr`，开发环境回退到仓库目录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrPaths {
    script: PathBuf,
    model: PathBuf,
}

impl AsrPaths {
    pub fn resolve(resource_dir: Option<&Path>) -> Self {
        let dev_root = Path::new(DEV_ROOT);
        if let Some(resource_dir) = resource_dir {
            let script = resource_dir.join("asr/stream_asr.py");
            let model = resource_dir.join("asr").join(MODEL_DIR_NAME);
            if script.is_file() && model.is_dir() {
                return Self { script, model };
            }
        }
        // 打包前把模型复制到 `app/src-tauri/asr/<model>`（不入库），
        // 与 `tauri.asr.conf.json` 的资源映射保持一致。
        let repository_script = dev_root.join("asr/stream_asr.py");
        let repository_model = dev_root.join("asr").join(MODEL_DIR_NAME);
        if repository_model.is_dir() {
            return Self {
                script: repository_script,
                model: repository_model,
            };
        }
        Self {
            script: repository_script,
            model: dev_root.join("../../.tmp/asr").join(MODEL_DIR_NAME),
        }
    }

    fn validate(&self) -> Result<(), IpcErrorDto> {
        if !self.script.is_file() || !self.model.is_dir() {
            return Err(IpcErrorDto::new(
                "asr_resource_missing",
                "实时转写资源缺失，请重新安装应用。",
                false,
            ));
        }
        Ok(())
    }
}

/// 实时转写管线状态，由 Tauri managed state 持有（`Arc<AsrPipeline>`）。
pub struct AsrPipeline {
    running: AtomicBool,
    generation: AtomicU64,
    last_text: Mutex<String>,
    last_error: Mutex<String>,
    child: Mutex<Option<Child>>,
    cards: Arc<CardStore>,
    paths: AsrPaths,
    subtitle: UiaSubtitleSource,
}

impl AsrPipeline {
    pub(crate) fn new(cards: Arc<CardStore>, paths: AsrPaths) -> Self {
        Self {
            running: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            last_text: Mutex::new(String::new()),
            last_error: Mutex::new(String::new()),
            child: Mutex::new(None),
            cards,
            paths,
            subtitle: UiaSubtitleSource::new(Platform::TencentMeeting),
        }
    }

    fn set_last_text(&self, text: &str) {
        if let Ok(mut last) = self.last_text.lock() {
            *last = text.to_owned();
        }
    }

    fn set_error(&self, error: &str) {
        if let Ok(mut last) = self.last_error.lock() {
            *last = error.to_owned();
        }
    }

    fn clear_error(&self) {
        self.set_error("");
    }

    pub fn last_error(&self) -> String {
        self.last_error
            .lock()
            .map(|error| error.clone())
            .unwrap_or_default()
    }

    /// 启动 UIA 字幕通道；重复调用无副作用。
    ///
    /// 字幕通道与 ASR 独立：会议被检测到时由 `meeting_status` 驱动启动，
    /// 手动开始实时转写时也会确保其运行，两者共用同一次订阅。
    pub fn ensure_subtitle(self: &Arc<Self>) {
        if self.subtitle.is_running() {
            return;
        }
        let cards = Arc::clone(&self.cards);
        let pipeline = Arc::downgrade(self);
        let sink = Box::new(move |event: SubtitleEvent| {
            let text = event.text.as_str();
            if let Some(pipeline) = pipeline.upgrade() {
                pipeline.set_last_text(text);
            }
            let _ = cards.ingest(text);
        });
        let _ = self.subtitle.subscribe(sink);
    }

    pub fn stop_subtitle(&self) {
        if self.subtitle.is_running() {
            self.subtitle.stop();
        }
    }

    pub fn subtitle_status(&self) -> &'static str {
        if !self.subtitle.is_running() {
            return "off";
        }
        match self.subtitle.health() {
            SourceHealth::Ready => "ready",
            SourceHealth::Degraded(_) => "degraded",
            SourceHealth::Unavailable(_) => "off",
        }
    }

    fn is_current(&self, generation: u64) -> bool {
        self.running.load(Ordering::SeqCst) && self.generation.load(Ordering::SeqCst) == generation
    }

    fn stop_generation(&self, generation: u64) {
        if self.generation.load(Ordering::SeqCst) == generation {
            self.stop();
        }
    }

    /// 启动实时转写；重复调用无副作用。
    pub fn start(self: &Arc<Self>) -> Result<(), IpcErrorDto> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        if let Err(error) = self.paths.validate() {
            self.set_error(&error.message);
            return Err(error);
        }
        self.clear_error();
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.ensure_subtitle();
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let spawned = spawn_recognizer(&self.paths);
        let (child, stdin, stdout) = match spawned {
            Ok(parts) => parts,
            Err(error) => {
                self.running.store(false, Ordering::SeqCst);
                self.set_error(&error.message);
                return Err(error);
            }
        };
        match self.child.lock() {
            Ok(mut slot) => *slot = Some(child),
            Err(_) => {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                self.running.store(false, Ordering::SeqCst);
                self.set_error("识别进程状态不可用。");
                return Err(IpcErrorDto::new(
                    "asr_state_unavailable",
                    "识别进程状态不可用。",
                    true,
                ));
            }
        }

        // 工作线程只持有 Weak，避免应用退出时线程反过来阻止管线 Drop。
        let recognizer = Arc::downgrade(self);
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let Some(recognizer) = recognizer.upgrade() else {
                    return;
                };
                if !recognizer.is_current(generation) {
                    return;
                }
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                let Some(text) = value.get("text").and_then(|item| item.as_str()) else {
                    continue;
                };
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }
                recognizer.set_last_text(text);
                let _ = recognizer.cards.ingest(text);
            }
            if let Some(recognizer) = recognizer.upgrade() {
                if recognizer.is_current(generation) {
                    // 用户没点停止却读到 EOF，说明识别进程自己退出了（缺 vosk、模型损坏等）。
                    recognizer.set_error(
                        "识别进程已退出：请确认 Python 3 已安装 vosk，且 ASR 模型可读。",
                    );
                }
                recognizer.stop_generation(generation);
            }
        });

        let capture = Arc::downgrade(self);
        std::thread::spawn(move || {
            let source = WasapiLoopbackSource::new(Platform::TencentMeeting);
            if source.open(None).is_err() {
                if let Some(capture) = capture.upgrade() {
                    if capture.is_current(generation) {
                        capture.set_error("系统音频采集不可用：请确认存在可用的默认输出设备。");
                    }
                    capture.stop_generation(generation);
                }
                return;
            }
            let mut writer = stdin;
            while capture
                .upgrade()
                .is_some_and(|capture| capture.is_current(generation))
            {
                match source.next_chunk() {
                    Ok(Some(chunk)) => {
                        let pcm = resample_to_16k(&chunk.samples, chunk.sample_rate);
                        if pcm.is_empty() {
                            continue;
                        }
                        let mut bytes = Vec::with_capacity(pcm.len() * 2);
                        for sample in pcm {
                            bytes.extend_from_slice(&sample.to_le_bytes());
                        }
                        if writer.write_all(&bytes).is_err() {
                            break;
                        }
                        let _ = writer.flush();
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                    Err(_) => std::thread::sleep(Duration::from_millis(50)),
                }
            }
            if let Some(capture) = capture.upgrade() {
                capture.stop_generation(generation);
            }
        });

        Ok(())
    }

    /// 停止实时转写并回收子进程。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.subtitle.stop();
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) fn set_running_for_test(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }

    pub fn last_text(&self) -> String {
        self.last_text
            .lock()
            .map(|text| text.clone())
            .unwrap_or_default()
    }
}

impl Drop for AsrPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

fn quiet_command(program: &str, prefix_args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command.args(prefix_args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// 候选解释器是否装了 vosk。
///
/// PATH 里第一个 `python` 未必是装了 vosk 的那个（例如 IDE/工具自带的嵌入式 Python），
/// 直接用它会让识别进程启动后立刻 ImportError 退出，表现为"开始转写但一直没有文本"。
fn python_has_vosk(program: &str, prefix_args: &[&str]) -> bool {
    quiet_command(program, prefix_args)
        .arg("-c")
        .arg("import vosk")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn spawn_recognizer(
    paths: &AsrPaths,
) -> Result<(Child, std::process::ChildStdin, std::process::ChildStdout), IpcErrorDto> {
    paths.validate()?;
    let candidates: [(&str, &[&str]); 3] = [("python", &[]), ("py", &["-3"]), ("python3", &[])];
    let mut checked: Vec<&str> = Vec::new();
    for (program, prefix_args) in candidates {
        if !python_has_vosk(program, prefix_args) {
            checked.push(program);
            continue;
        }
        let mut command = quiet_command(program, prefix_args);
        command
            .arg(&paths.script)
            .arg(&paths.model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                checked.push(program);
                continue;
            }
            Err(error) => {
                return Err(IpcErrorDto::new(
                    "asr_spawn_failed",
                    &format!("无法启动识别进程：{error}"),
                    true,
                ));
            }
        };
        let Some(stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(IpcErrorDto::new(
                "asr_spawn_failed",
                "识别进程输入不可用。",
                true,
            ));
        };
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(IpcErrorDto::new(
                "asr_spawn_failed",
                "识别进程输出不可用。",
                true,
            ));
        };
        return Ok((child, stdin, stdout));
    }
    let detail = checked.join(" / ");
    Err(IpcErrorDto::new(
        "asr_python_missing",
        &format!("未找到已安装 vosk 的 Python 3（已检查 {detail}），请先 pip install vosk。"),
        false,
    ))
}

/// 把任意采样率的单声道 f32 下采样为 16 kHz s16le。
pub fn resample_to_16k(samples: &[f32], source_rate: u32) -> Vec<i16> {
    let to_i16 = |value: f32| -> i16 {
        let clamped = value.clamp(-1.0, 1.0);
        (clamped * f32::from(i16::MAX)) as i16
    };
    if source_rate == 0 || samples.is_empty() {
        return Vec::new();
    }
    if source_rate == 16_000 {
        return samples.iter().copied().map(to_i16).collect();
    }
    let ratio = f64::from(source_rate) / 16_000.0;
    let out_len = (samples.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for index in 0..out_len {
        let start = (index as f64 * ratio).floor() as usize;
        let end = (((index + 1) as f64) * ratio).floor() as usize;
        let end = end.min(samples.len());
        if start >= end {
            continue;
        }
        let sum: f32 = samples[start..end].iter().sum();
        out.push(to_i16(sum / (end - start) as f32));
    }
    out
}

#[tauri::command]
pub fn start_asr(state: tauri::State<'_, Arc<AsrPipeline>>) -> Result<(), IpcErrorDto> {
    state.inner().start()
}

#[tauri::command]
pub fn stop_asr(state: tauri::State<'_, Arc<AsrPipeline>>) -> Result<(), IpcErrorDto> {
    state.stop();
    Ok(())
}

#[tauri::command]
pub fn asr_status(
    state: tauri::State<'_, Arc<AsrPipeline>>,
) -> Result<(bool, String, String), IpcErrorDto> {
    Ok((state.is_running(), state.last_text(), state.last_error()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline() -> AsrPipeline {
        AsrPipeline::new(Arc::new(CardStore::new()), AsrPaths::resolve(None))
    }

    #[test]
    fn stop_resets_running_state_without_a_child() {
        let pipeline = pipeline();
        pipeline.running.store(true, Ordering::SeqCst);
        pipeline.stop();
        assert!(!pipeline.is_running());
    }

    #[test]
    fn stale_worker_cannot_stop_a_newer_generation() {
        let pipeline = pipeline();
        pipeline.running.store(true, Ordering::SeqCst);
        pipeline.generation.store(2, Ordering::SeqCst);
        pipeline.stop_generation(1);
        assert!(pipeline.is_running());
        pipeline.stop_generation(2);
        assert!(!pipeline.is_running());
    }

    #[test]
    fn downsamples_48k_to_16k() {
        let samples = vec![0.5f32; 48_000];
        let pcm = resample_to_16k(&samples, 48_000);
        assert_eq!(pcm.len(), 16_000);
        assert!(pcm.iter().all(|sample| *sample > 16_000));
    }

    #[test]
    fn keeps_16k_passthrough() {
        let samples = vec![-1.0f32, 0.0, 1.0];
        let pcm = resample_to_16k(&samples, 16_000);
        assert_eq!(pcm.len(), 3);
        assert_eq!(pcm[1], 0);
    }

    #[test]
    fn missing_program_is_not_reported_as_vosk_ready() {
        assert!(!python_has_vosk("definitely-not-a-python-under-test", &[]));
    }

    #[test]
    fn error_slot_starts_empty() {
        assert_eq!(pipeline().last_error(), "");
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(resample_to_16k(&[], 48_000).is_empty());
        assert!(resample_to_16k(&[0.5], 0).is_empty());
    }
}
