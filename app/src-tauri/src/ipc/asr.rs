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
use crate::ipc::transcript::TranscriptLog;

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

/// 识别进程 stdout 的一行：中间结果只做展示，稳定句才出卡。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recognized {
    Partial(String),
    Stable(String),
}

/// 实时转写管线状态，由 Tauri managed state 持有（`Arc<AsrPipeline>`）。
pub struct AsrPipeline {
    running: AtomicBool,
    generation: AtomicU64,
    last_text: Mutex<String>,
    /// UIA 字幕通道的文本，与 ASR 分开存放：ASR 有内容时优先展示 ASR。
    subtitle_text: Mutex<String>,
    last_error: Mutex<String>,
    child: Mutex<Option<Child>>,
    cards: Arc<CardStore>,
    /// 本次会话的转写全文；稳定句入栈，供用户回看与选中查询。
    transcripts: Arc<TranscriptLog>,
    paths: AsrPaths,
    subtitle: UiaSubtitleSource,
    /// 诊断计数：用来区分"没采到音频"和"采到了但没识别"。
    audio_chunks: AtomicU64,
    audio_bytes: AtomicU64,
    audio_ms: AtomicU64,
    stable_lines: AtomicU64,
    last_stable: Mutex<String>,
    /// 转写时是否自动出卡。默认关：开会时每句都出卡会把卡片区刷成噪音，
    /// 用户要的是"转写成文字 → 自己选中 → 查意思 → 需要时才出卡"。
    auto_cards: AtomicBool,
}

impl AsrPipeline {
    pub(crate) fn new(
        cards: Arc<CardStore>,
        transcripts: Arc<TranscriptLog>,
        paths: AsrPaths,
    ) -> Self {
        Self {
            running: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            last_text: Mutex::new(String::new()),
            subtitle_text: Mutex::new(String::new()),
            last_error: Mutex::new(String::new()),
            child: Mutex::new(None),
            cards,
            transcripts,
            paths,
            subtitle: UiaSubtitleSource::new(Platform::TencentMeeting),
            audio_chunks: AtomicU64::new(0),
            audio_bytes: AtomicU64::new(0),
            audio_ms: AtomicU64::new(0),
            stable_lines: AtomicU64::new(0),
            last_stable: Mutex::new(String::new()),
            auto_cards: AtomicBool::new(false),
        }
    }

    /// 自动出卡开关（默认关）。
    pub fn auto_cards(&self) -> bool {
        self.auto_cards.load(Ordering::Relaxed)
    }

    pub fn set_auto_cards(&self, enabled: bool) {
        self.auto_cards.store(enabled, Ordering::Relaxed);
    }

    fn set_last_text(&self, text: &str) {
        if let Ok(mut last) = self.last_text.lock() {
            *last = text.to_owned();
        }
    }

    fn set_subtitle_text(&self, text: &str) {
        if let Ok(mut last) = self.subtitle_text.lock() {
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
            let Some(pipeline) = pipeline.upgrade() else {
                return;
            };
            // 字幕通道独立存放：不能让 UIA 文本覆盖 ASR 的实时文本。
            pipeline.set_subtitle_text(text);
            // 只有用户显式打开"自动出卡"时才从这里出卡，否则卡片区会被字幕刷屏。
            if pipeline.auto_cards() {
                let _ = cards.ingest(text);
            }
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
        self.reset_counters();
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        crate::diag::log(&format!(
            "start script={} model={}",
            self.paths.script.display(),
            self.paths.model.display()
        ));
        let spawned = spawn_recognizer(&self.paths);
        let (child, stdin, stdout) = match spawned {
            Ok(parts) => parts,
            Err(error) => {
                self.running.store(false, Ordering::SeqCst);
                self.set_error(&error.message);
                crate::diag::log(&format!(
                    "start.failed code={} msg={}",
                    error.code, error.message
                ));
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
            let mut reader = BufReader::new(stdout);
            let mut raw = Vec::new();
            let mut lines = 0usize;
            loop {
                raw.clear();
                match reader.read_until(b'\n', &mut raw) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
                lines += 1;
                if lines.is_multiple_of(200) {
                    crate::diag::log(&format!("recognizer.lines={lines}"));
                }
                let Some(recognizer) = recognizer.upgrade() else {
                    return;
                };
                if !recognizer.is_current(generation) {
                    return;
                }
                // 脚本已固定 UTF-8；这里仍用 lossy，避免一个坏字节终结整条流。
                let line = String::from_utf8_lossy(&raw);
                let Some(recognized) = parse_line(&line) else {
                    continue;
                };
                match recognized {
                    // 中间结果只更新界面文本，不抽关键词：半句会产出碎片卡片。
                    Recognized::Partial(text) => recognizer.set_last_text(&text),
                    Recognized::Stable(text) => {
                        recognizer.set_last_text(&text);
                        recognizer.stable_lines.fetch_add(1, Ordering::Relaxed);
                        // 稳定句进全文缓冲：用户回看与选中查询都以这里为准。
                        recognizer.transcripts.push(&text);
                        // 去重副作用必须保留（脚本收句会重复上一句）；
                        // 但只有显式开启自动出卡时才真的往卡片区塞卡片。
                        let fresh = recognizer.take_stable_if_new(&text);
                        if fresh && recognizer.auto_cards() {
                            let _ = recognizer.cards.ingest(&text);
                        }
                    }
                }
            }
            if let Some(recognizer) = recognizer.upgrade() {
                if recognizer.is_current(generation) {
                    // 用户没点停止却读到 EOF，说明识别进程自己退出了（缺 vosk、模型损坏等）。
                    recognizer.set_error(
                        "识别进程已退出：请确认 Python 3 已安装 vosk，且 ASR 模型可读。",
                    );
                    crate::diag::log(&format!(
                        "recognizer.eof lines={lines} chunks={} stable={}",
                        recognizer.audio_chunks.load(Ordering::Relaxed),
                        recognizer.stable_lines.load(Ordering::Relaxed)
                    ));
                }
                recognizer.stop_generation(generation);
            }
        });

        let capture = Arc::downgrade(self);
        std::thread::spawn(move || {
            let source = WasapiLoopbackSource::new(Platform::TencentMeeting);
            if let Err(error) = source.open(None) {
                crate::diag::log(&format!("capture.open_failed {error:?}"));
                if let Some(capture) = capture.upgrade() {
                    if capture.is_current(generation) {
                        capture.set_error("系统音频采集不可用：请确认存在可用的默认输出设备。");
                    }
                    capture.stop_generation(generation);
                }
                return;
            }
            crate::diag::log(&format!("capture.opened health={:?}", source.health()));
            let mut writer = stdin;
            let mut idle = 0u64;
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
                        let pcm_ms = u64::try_from(bytes.len()).unwrap_or(0) / 32;
                        if writer.write_all(&bytes).is_err() {
                            crate::diag::log("capture.write_failed");
                            break;
                        }
                        let _ = writer.flush();
                        idle = 0;
                        if let Some(capture) = capture.upgrade() {
                            let chunks = capture.audio_chunks.fetch_add(1, Ordering::Relaxed) + 1;
                            capture
                                .audio_bytes
                                .fetch_add(bytes.len() as u64, Ordering::Relaxed);
                            capture.audio_ms.fetch_add(pcm_ms, Ordering::Relaxed);
                            if chunks.is_multiple_of(200) {
                                crate::diag::log(&format!(
                                    "capture.chunks={chunks} audio={:.1}s stable={}",
                                    capture.audio_ms.load(Ordering::Relaxed) as f64 / 1000.0,
                                    capture.stable_lines.load(Ordering::Relaxed)
                                ));
                            }
                        }
                    }
                    Ok(None) => {
                        idle += 1;
                        if idle.is_multiple_of(500) {
                            // 每 10 秒记一次空转，用于区分"没有声音"与"识别不出"。
                            crate::diag::log(&format!("capture.idle={}s", idle / 50));
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => {
                        crate::diag::log(&format!("capture.next_chunk_failed {error:?}"));
                        std::thread::sleep(Duration::from_millis(50));
                    }
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

    /// 界面文本位：ASR 有实时文本就用 ASR，否则回退到 UIA 字幕通道。
    pub fn display_text(&self) -> String {
        let asr = self.last_text();
        if !asr.is_empty() {
            return asr;
        }
        self.subtitle_text
            .lock()
            .map(|text| text.clone())
            .unwrap_or_default()
    }

    /// 采集/识别计数，供界面区分"没有声音进管线"与"有声音但没识别出文本"。
    pub fn diagnostics(&self) -> String {
        let chunks = self.audio_chunks.load(Ordering::Relaxed);
        let bytes = self.audio_bytes.load(Ordering::Relaxed);
        let ms = self.audio_ms.load(Ordering::Relaxed);
        let stable = self.stable_lines.load(Ordering::Relaxed);
        if chunks == 0 {
            // 0 片必须显式说出来：否则界面只剩"等待说话"，无法区分"没声音"与"识别不出"。
            return "音频 0.0s（0 片）：未采到系统声音".to_owned();
        }
        format!(
            "音频 {:.1}s（{chunks} 片）/ 写入 {:.0}KB / 稳定句 {stable}",
            ms as f64 / 1000.0,
            bytes as f64 / 1024.0
        )
    }

    fn reset_counters(&self) {
        self.audio_chunks.store(0, Ordering::Relaxed);
        self.audio_bytes.store(0, Ordering::Relaxed);
        self.audio_ms.store(0, Ordering::Relaxed);
        self.stable_lines.store(0, Ordering::Relaxed);
        if let Ok(mut last) = self.last_stable.lock() {
            last.clear();
        }
    }

    /// 稳定句去重：脚本强制收句时可能重复上一句，避免刷重复卡片。
    fn take_stable_if_new(&self, text: &str) -> bool {
        match self.last_stable.lock() {
            Ok(last) if last.as_str() == text => false,
            Ok(mut last) => {
                *last = text.to_owned();
                true
            }
            Err(_) => true,
        }
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
    // 子进程 stdout 走管道时，Windows 上 Python 可能退回 cp936；固定 UTF-8 以免中文乱码或解码失败。
    command
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// 解析识别进程的一行输出；`{"partial"}` 为中间结果，`{"text"}` 为稳定句。
pub fn parse_line(line: &str) -> Option<Recognized> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if let Some(text) = value.get("text").and_then(|item| item.as_str()) {
        let text = text.trim();
        if !text.is_empty() {
            return Some(Recognized::Stable(text.to_owned()));
        }
    }
    let partial = value.get("partial").and_then(|item| item.as_str())?.trim();
    if partial.is_empty() {
        None
    } else {
        Some(Recognized::Partial(partial.to_owned()))
    }
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
            crate::diag::log(&format!("python.no_vosk {program}"));
            continue;
        }
        crate::diag::log(&format!("python.vosk_ok {program} {prefix_args:?}"));
        let mut command = quiet_command(program, prefix_args);
        // -X utf8 与 PYTHONUTF8 双保险：部分发行版忽略环境变量。
        command
            .arg("-X")
            .arg("utf8")
            .arg(&paths.script)
            .arg(&paths.model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
        crate::diag::log(&format!("spawn.ok pid={}", child.id()));
        // stderr 保留并转存诊断日志：缺模型、缺依赖、内存不足都会先在这里留下痕迹。
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let mut lines = 0usize;
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    lines += 1;
                    if lines <= 20 {
                        let text: String = line.chars().take(200).collect();
                        crate::diag::log(&format!("py.stderr {text}"));
                    }
                }
                if lines > 20 {
                    crate::diag::log(&format!("py.stderr.total={lines}"));
                }
            });
        }
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
) -> Result<(bool, String, String, String, bool), IpcErrorDto> {
    Ok((
        state.is_running(),
        state.display_text(),
        state.last_error(),
        state.diagnostics(),
        state.auto_cards(),
    ))
}

/// 转写时是否自动出卡。
///
/// 默认关：每句稳定文本都抽关键词会产出大量碎片卡片，把卡片区刷成噪音；
/// 用户要的路径是"转写成文字 → 自己选中 → 查意思 → 需要时才出卡"。
#[tauri::command]
pub fn set_auto_cards(
    state: tauri::State<'_, Arc<AsrPipeline>>,
    enabled: bool,
) -> Result<bool, IpcErrorDto> {
    state.set_auto_cards(enabled);
    Ok(state.auto_cards())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_application::settings::CardSnapshotSource;

    fn pipeline() -> AsrPipeline {
        AsrPipeline::new(
            Arc::new(CardStore::new()),
            Arc::new(TranscriptLog::new()),
            AsrPaths::resolve(None),
        )
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
    fn parses_stable_and_partial_lines() {
        assert_eq!(
            parse_line(r#"{"text": "灰度范围限定"}"#),
            Some(Recognized::Stable("灰度范围限定".to_owned()))
        );
        assert_eq!(
            parse_line(r#"{"partial": "灰度 范围"}"#),
            Some(Recognized::Partial("灰度 范围".to_owned()))
        );
    }

    #[test]
    fn ignores_blank_and_malformed_lines() {
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("not json"), None);
        assert_eq!(parse_line(r#"{"text": "   "}"#), None);
        assert_eq!(parse_line(r#"{"partial": ""}"#), None);
        assert_eq!(parse_line(r#"{"other": 1}"#), None);
    }

    /// 部分收敛：只有 partial 也应推送到界面文本位（否则会议全程"等待说话"）。
    #[test]
    fn partial_updates_visible_text_without_cards() {
        let pipeline = pipeline();
        if let Some(Recognized::Partial(text)) = parse_line(r#"{"partial": "灰度 范围"}"#) {
            pipeline.set_last_text(&text);
        }
        assert_eq!(pipeline.last_text(), "灰度 范围");
        assert!(pipeline.cards.cards().is_empty(), "中间结果不直接出卡");
    }

    #[test]
    fn repeated_stable_sentence_is_ingested_once() {
        let pipeline = pipeline();
        assert!(pipeline.take_stable_if_new("灰度范围限定"));
        assert!(!pipeline.take_stable_if_new("灰度范围限定"));
        assert!(pipeline.take_stable_if_new("两地三中心"));
    }

    /// 自动出卡默认关：开会时只积累转写文字，卡片区不该自己长卡片。
    #[test]
    fn auto_cards_default_off_and_switchable() {
        let pipeline = pipeline();
        assert!(!pipeline.auto_cards(), "默认不自动出卡");
        pipeline.set_auto_cards(true);
        assert!(pipeline.auto_cards());
        pipeline.set_auto_cards(false);
        assert!(!pipeline.auto_cards());
    }

    #[test]
    fn diagnostics_report_zero_chunks_explicitly() {
        let pipeline = pipeline();
        let text = pipeline.diagnostics();
        // 0 片时不能返回空串：界面需要能区分"没采到声音"与"识别不出"。
        assert!(text.contains("0 片"), "{text}");
        assert!(text.contains("未采到系统声音"), "{text}");
        pipeline.audio_chunks.store(3, Ordering::Relaxed);
        pipeline.audio_bytes.store(3200, Ordering::Relaxed);
        pipeline.audio_ms.store(500, Ordering::Relaxed);
        let text = pipeline.diagnostics();
        assert!(text.contains("0.5s"), "{text}");
        assert!(text.contains("3 片"), "{text}");
    }

    /// UIA 字幕不得覆盖 ASR 实时文本；ASR 空时才回退字幕。
    #[test]
    fn asr_text_wins_over_subtitle_text() {
        let pipeline = pipeline();
        pipeline.set_subtitle_text("字幕通道文本");
        assert_eq!(pipeline.display_text(), "字幕通道文本");
        pipeline.set_last_text("我们确认灰度范围");
        assert_eq!(pipeline.display_text(), "我们确认灰度范围");
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(resample_to_16k(&[], 48_000).is_empty());
        assert!(resample_to_16k(&[0.5], 0).is_empty());
    }
}
