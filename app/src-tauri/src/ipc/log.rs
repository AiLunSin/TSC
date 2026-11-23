//! 日志入口：定位应用日志文件并在资源管理器中选中。

use tauri::{AppHandle, Manager};

use crate::ipc::dto::IpcErrorDto;

fn log_path(app: &AppHandle) -> Result<std::path::PathBuf, IpcErrorDto> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|_| IpcErrorDto::new("log_unavailable", "日志目录不可用。", true))?;
    std::fs::create_dir_all(&dir)
        .map_err(|_| IpcErrorDto::new("log_unavailable", "日志目录不可用。", true))?;
    let path = dir.join("desktop.log");
    if !path.exists() {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| IpcErrorDto::new("log_unavailable", "日志文件不可用。", true))?;
    }
    Ok(path)
}

#[tauri::command]
pub fn open_log(app: AppHandle) -> Result<String, IpcErrorDto> {
    let path = log_path(&app)?;
    let selection = format!("/select,{}", path.display());
    std::process::Command::new("explorer.exe")
        .arg(selection)
        .spawn()
        .map_err(|_| IpcErrorDto::new("log_open_failed", "无法打开日志文件。", true))?;
    Ok(path.display().to_string())
}
