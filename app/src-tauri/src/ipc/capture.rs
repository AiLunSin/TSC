//! 采集通道状态：只读投影；系统音频来自 WASAPI，字幕与 ASR 仍为未实现。

use crate::ipc::dto::IpcErrorDto;

#[tauri::command]
pub fn capture_status() -> Result<Vec<(String, String)>, IpcErrorDto> {
    Ok(vec![
        ("字幕文本通道".to_owned(), "off".to_owned()),
        (
            "系统音频通道".to_owned(),
            meeting_copilot_platform_win::wasapi::system_audio_status().to_owned(),
        ),
        ("本地 ASR".to_owned(), "off".to_owned()),
    ])
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_audio_status_from_platform_layer() {
        let rows = capture_status().expect("capture status");
        assert_eq!(rows.len(), 3);
        let audio = &rows[1].1;
        assert!(
            audio == "ready" || audio == "degraded" || audio == "off",
            "unexpected audio state: {audio}"
        );
        eprintln!("system_audio={audio}");
    }
}
