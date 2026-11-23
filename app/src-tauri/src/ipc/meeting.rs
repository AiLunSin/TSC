//! 会议检测：按进程名识别已知会议客户端，返回显示名与窗口标题。

use crate::ipc::dto::IpcErrorDto;

#[tauri::command]
pub fn meeting_status() -> Result<Option<(String, String)>, IpcErrorDto> {
    Ok(meeting_copilot_platform_win::meeting::detect_running()
        .map(|(app, title)| (app.to_owned(), title)))
}
