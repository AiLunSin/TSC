//! 实时转写管线：WASAPI 环回 → 16 kHz PCM → vosk 流式 → 关键词卡片。
//!
//! Python 子进程只做识别；采集、重采样与出卡都在进程内完成。

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use meeting_copilot_domain::model::enums::Platform;
use meeting_copilot_domain::port::capture::AudioSource;
use meeting_copilot_platform_win::wasapi::WasapiLoopbackSource;

use crate::ipc::card_store::card_store;
use crate::ipc::dto::IpcErrorDto;

const SCRIPT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/asr/stream_asr.py");
const MODEL_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../.tmp/asr/vosk-model-small-cn-0.22"
);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct Pipeline {
    running: AtomicBool,
    last_text: Mutex<String>,
    child: Mutex<Option<Child>>,
}

fn pipeline() -> &'static Pipeline {
    static PIPELINE: OnceLock<Pipeline> = OnceLock::new();
    PIPELINE.get_or_init(|| Pipeline {
        running: AtomicBool::new(false),
        last_text: Mutex::new(String::new()),
        child: Mutex::new(None),
    })
}

fn set_last_text(text: &str) {
    if let Ok(mut last) = pipeline().last_text.lock() {
        *last = text.to_owned();
    }
}

/// 启动实时转写；重复调用无副作用。
pub fn start() -> Result<(), IpcErrorDto> {
    let state = pipeline();
    if state.running.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let spawned = spawn_recognizer();
    let (child, stdin, stdout) = match spawned {
        Ok(parts) => parts,
        Err(error) => {
            state.running.store(false, Ordering::SeqCst);
            return Err(error);
        }
    };
    if let Ok(mut slot) = state.child.lock() {
        *slot = Some(child);
    }

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
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
            set_last_text(text);
            let _ = card_store().ingest(text);
        }
    });

    std::thread::spawn(move || {
        let source = WasapiLoopbackSource::new(Platform::TencentMeeting);
        if source.open(None).is_err() {
            stop();
            return;
        }
        let mut writer = stdin;
        while pipeline().running.load(Ordering::SeqCst) {
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
    });

    Ok(())
}

/// 停止实时转写并回收子进程。
pub fn stop() {
    let state = pipeline();
    state.running.store(false, Ordering::SeqCst);
    if let Ok(mut slot) = state.child.lock() {
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn is_running() -> bool {
    pipeline().running.load(Ordering::SeqCst)
}

pub fn last_text() -> String {
    pipeline()
        .last_text
        .lock()
        .map(|text| text.clone())
        .unwrap_or_default()
}

fn spawn_recognizer(
) -> Result<(Child, std::process::ChildStdin, std::process::ChildStdout), IpcErrorDto> {
    let mut command = Command::new("python");
    command
        .arg(SCRIPT_PATH)
        .arg(MODEL_PATH)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|error| {
        IpcErrorDto::new(
            "asr_spawn_failed",
            &format!("无法启动识别进程：{error}"),
            true,
        )
    })?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| IpcErrorDto::new("asr_spawn_failed", "识别进程输入不可用。", true))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| IpcErrorDto::new("asr_spawn_failed", "识别进程输出不可用。", true))?;
    Ok((child, stdin, stdout))
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
pub fn start_asr() -> Result<(), IpcErrorDto> {
    start()
}

#[tauri::command]
pub fn stop_asr() -> Result<(), IpcErrorDto> {
    stop();
    Ok(())
}

#[tauri::command]
pub fn asr_status() -> Result<(bool, String), IpcErrorDto> {
    Ok((is_running(), last_text()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn empty_input_yields_empty_output() {
        assert!(resample_to_16k(&[], 48_000).is_empty());
        assert!(resample_to_16k(&[0.5], 0).is_empty());
    }
}
