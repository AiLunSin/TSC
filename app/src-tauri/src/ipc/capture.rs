//! 采集通道状态：只读投影；系统音频来自 WASAPI，ASR 来自实时管线。

use std::sync::Arc;

use crate::ipc::asr::AsrPipeline;
use crate::ipc::dto::IpcErrorDto;

pub fn capture_status_impl(asr: &AsrPipeline) -> Vec<(String, String)> {
    vec![
        ("字幕文本通道".to_owned(), asr.subtitle_status().to_owned()),
        (
            "系统音频通道".to_owned(),
            meeting_copilot_platform_win::wasapi::system_audio_status().to_owned(),
        ),
        (
            "本地 ASR".to_owned(),
            if asr.is_running() { "ready" } else { "off" }.to_owned(),
        ),
    ]
}

#[tauri::command]
pub fn capture_status(
    state: tauri::State<'_, Arc<AsrPipeline>>,
) -> Result<Vec<(String, String)>, IpcErrorDto> {
    Ok(capture_status_impl(state.inner()))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_audio_status_from_platform_layer() {
        let asr = AsrPipeline::new(
            Arc::new(crate::ipc::card_store::CardStore::new()),
            Arc::new(crate::ipc::transcript::TranscriptLog::new()),
            crate::ipc::asr::AsrPaths::resolve(None),
        );
        let rows = capture_status_impl(&asr);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].1, "off", "字幕通道未启动时应为 off");
        let audio = &rows[1].1;
        assert!(
            audio == "ready" || audio == "degraded" || audio == "off",
            "unexpected audio state: {audio}"
        );
        assert_eq!(rows[2].1, "off");
    }

    #[test]
    fn reports_running_asr_as_ready() {
        let asr = AsrPipeline::new(
            Arc::new(crate::ipc::card_store::CardStore::new()),
            Arc::new(crate::ipc::transcript::TranscriptLog::new()),
            crate::ipc::asr::AsrPaths::resolve(None),
        );
        asr.set_running_for_test(true);
        let rows = capture_status_impl(&asr);
        assert_eq!(rows[2].1, "ready");
    }
}
