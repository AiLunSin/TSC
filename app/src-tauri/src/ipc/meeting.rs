//! 会议检测：按进程名识别已知会议客户端，只返回显示名。

use crate::ipc::dto::IpcErrorDto;

#[tauri::command]
pub fn meeting_status() -> Result<Option<String>, IpcErrorDto> {
    Ok(meeting_copilot_platform_win::meeting::detect_running().map(str::to_owned))
}
