//! 运行期诊断日志：只写本地文件，用于排查"开始转写但没有文本"这类环境问题。
//!
//! 边界：**不记录任何识别文本、会议内容、凭据或设备名**，只记录状态、计数与错误类型。
//! 目的是把"没有文本"拆成可区分的几类：资源缺失 / 解释器缺 vosk / 进程退出 /
//! 采集不到音频 / 有音频但识别不出稳定句。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

fn path() -> Option<&'static PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
        let dir = base.join("meeting-copilot");
        fs::create_dir_all(&dir).ok()?;
        Some(dir.join("asr.log"))
    })
    .as_ref()
}

/// 追加一行诊断日志；任何失败都静默忽略，不能因日志影响转写本身。
pub fn log(event: &str) {
    let Some(path) = path() else {
        return;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let _ = writeln!(file, "{stamp} {event}");
}
