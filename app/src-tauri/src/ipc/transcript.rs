//! 本次会话的转写全文：只保存在内存，停止转写或退出应用即清空。
//!
//! 与 `CardStore` 的区别：卡片是"从文本里提炼出的要点"，本模块保留**原始句子**，
//! 供用户回看、选中并查询含义。不落库、不写日志，避免会议内容留痕。

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::ipc::dto::TranscriptLineDto;

/// 单次会话最多保留的行数：约 600 句，足够一场长会，且不会无限占用内存。
const MAX_LINES: usize = 600;

/// 转写全文缓冲。
#[derive(Debug, Default)]
pub struct TranscriptLog {
    lines: Mutex<Vec<TranscriptLineDto>>,
}

impl TranscriptLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一句稳定句；空行与紧邻重复句直接忽略。
    pub fn push(&self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let Ok(mut lines) = self.lines.lock() else {
            return;
        };
        if lines.last().is_some_and(|last| last.text == trimmed) {
            return;
        }
        let index = lines.last().map_or(1, |last| last.index + 1);
        lines.push(TranscriptLineDto {
            index,
            text: trimmed.to_owned(),
            at_ms: now_ms(),
        });
        if lines.len() > MAX_LINES {
            lines.remove(0);
        }
    }

    /// 当前全文快照。
    pub fn snapshot(&self) -> Vec<TranscriptLineDto> {
        self.lines
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }

    /// 选中文字在全文里的上下文：前一句 + 后一句，供释义时判断语境。
    pub fn context_around(&self, needle: &str) -> String {
        let Ok(lines) = self.lines.lock() else {
            return String::new();
        };
        let Some(position) = lines.iter().position(|line| line.text.contains(needle)) else {
            return String::new();
        };
        let start = position.saturating_sub(1);
        let end = (position + 2).min(lines.len());
        lines[start..end]
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 当前会话的转写全文；前端按需拉取，用于展示与选中。
#[tauri::command]
pub fn transcript_snapshot(
    state: State<'_, std::sync::Arc<TranscriptLog>>,
) -> Result<Vec<TranscriptLineDto>, crate::ipc::dto::IpcErrorDto> {
    Ok(state.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_and_indexes_lines() {
        let log = TranscriptLog::new();
        log.push("第一句");
        log.push("第二句");
        let lines = log.snapshot();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].index, 1);
        assert_eq!(lines[1].index, 2);
        assert_eq!(lines[1].text, "第二句");
    }

    #[test]
    fn ignores_blank_and_repeated_lines() {
        let log = TranscriptLog::new();
        log.push("同一句");
        log.push("同一句");
        log.push("   ");
        assert_eq!(log.snapshot().len(), 1);
    }

    #[test]
    fn context_includes_neighbours() {
        let log = TranscriptLog::new();
        log.push("前面一句");
        log.push("这里提到了灰度发布");
        log.push("后面一句");
        let context = log.context_around("灰度");
        assert!(context.contains("前面一句"), "{context}");
        assert!(context.contains("灰度发布"), "{context}");
        assert!(context.contains("后面一句"), "{context}");
    }
}
