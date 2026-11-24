//! 会议检测：按进程名识别已知会议客户端，返回显示名与窗口标题。
//!
//! 检测同时驱动 UIA 字幕通道：检测到会议就确保字幕轮询在跑，
//! 会议结束后停掉，左侧"字幕文本通道"因此反映真实健康度而不是恒为未启用。

use std::sync::Arc;

use crate::ipc::asr::AsrPipeline;
use crate::ipc::dto::IpcErrorDto;

#[tauri::command]
pub fn meeting_status(
    state: tauri::State<'_, Arc<AsrPipeline>>,
) -> Result<Option<(String, String)>, IpcErrorDto> {
    match meeting_copilot_platform_win::meeting::detect_running() {
        Some((app, title)) => {
            state.ensure_subtitle();
            Ok(Some((app.to_owned(), title)))
        }
        None => {
            state.stop_subtitle();
            Ok(None)
        }
    }
}
