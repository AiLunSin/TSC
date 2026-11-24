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
        self.context_for(needle)
    }

    /// 选中文字（可能跨多句）的上下文。
    ///
    /// 前端把选区里的换行压成了空格，跨句选区是一整串；整段匹配不到时必须退化成
    /// "按最长的一句 / 最长的连续词块找位置"，否则跨句选中永远拿不到上下文。
    pub fn context_for(&self, selection: &str) -> String {
        let Ok(lines) = self.lines.lock() else {
            return String::new();
        };
        let Some(position) = locate(&lines, selection) else {
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

    /// 选中文字在转写里出现的行数：与 `context_for` 用同一匹配口径，
    /// 避免"上下文有、计数为 0"这种自相矛盾的回退文案。
    pub fn hits(&self, selection: &str) -> usize {
        let Ok(lines) = self.lines.lock() else {
            return 0;
        };
        let Some(key) = match_key(&lines, selection) else {
            return 0;
        };
        lines
            .iter()
            .filter(|line| line.text.contains(key.as_str()))
            .count()
    }
}

fn locate(lines: &[TranscriptLineDto], selection: &str) -> Option<usize> {
    let key = match_key(lines, selection)?;
    lines
        .iter()
        .position(|line| line.text.contains(key.as_str()))
}

/// 匹配键：整段 → 单句（长句优先）→ 连续词块（长块优先）。
///
/// ASR 的稳定句带空格（"灰度 范围 限定"），跨行选中时既匹配不到整段，
/// 也可能没有句读可切，只能靠词块窗口兜底。
fn match_key(lines: &[TranscriptLineDto], selection: &str) -> Option<String> {
    let whole = selection.trim();
    if whole.is_empty() {
        return None;
    }
    if lines.iter().any(|line| line.text.contains(whole)) {
        return Some(whole.to_owned());
    }
    let mut sentences = split_sentences(whole);
    sentences.sort_by_key(|sentence| std::cmp::Reverse(sentence.chars().count()));
    if let Some(hit) = sentences
        .into_iter()
        .find(|sentence| sentence.chars().count() >= 2 && contains_any(lines, sentence))
    {
        return Some(hit.to_owned());
    }
    let tokens: Vec<&str> = whole.split_whitespace().collect();
    let widest = tokens.len().min(8);
    for width in (2..=widest).rev() {
        for chunk in tokens.windows(width) {
            let key = chunk.join(" ");
            if contains_any(lines, &key) {
                return Some(key);
            }
        }
    }
    None
}

fn contains_any(lines: &[TranscriptLineDto], needle: &str) -> bool {
    lines.iter().any(|line| line.text.contains(needle))
}

/// 按中英文句读切分选中文字；片段已去空白，空片段丢弃。
pub fn split_sentences(text: &str) -> Vec<&str> {
    text.split(['。', '！', '？', '；', '!', '?', ';', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect()
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

/// 去掉**两个中文片段之间**的空格。
///
/// vosk 的中文结果是按词带空格的（`学历 歧视 的 现象`），直接展示像被切碎，
/// 抽关键词和选中查询也会被空格干扰。这里只在"上一个字和下一个字都是中文/中文标点"
/// 时丢掉空格：英文短语（`degree discrimination`）、中英混排（`SLA 指标`）保持原样。
pub fn tidy_cjk_spaces(text: &str) -> String {
    fn joins(c: char) -> bool {
        matches!(c as u32,
            0x3000..=0x303F   // 中文标点（、。〈〉《》「」…）
            | 0x3400..=0x4DBF // 扩展 A
            | 0x4E00..=0x9FFF // 基本汉字
            | 0xF900..=0xFAFF // 兼容汉字
            | 0xFF01..=0xFF5E // 全角字符与标点
            | 0xFFE0..=0xFFE6 // 全角货币等符号
        )
    }
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            pending_space = false;
            let both_cjk = out.chars().next_back().is_some_and(joins) && joins(ch);
            if !both_cjk {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
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

    /// 跨行选中（前端把换行压成空格）必须仍能定位到位置。
    #[test]
    fn context_survives_cross_line_selection() {
        let log = TranscriptLog::new();
        log.push("第一行 灰度 范围 限定 华中");
        log.push("第二行 我们 先 看 数据");
        log.push("第三行 结论 是 通过");
        let context = log.context_for("灰度 范围 限定 华中 我们 先 看 数据");
        assert!(context.contains("灰度"), "{context}");
        assert!(context.contains("第二行"), "{context}");
    }

    /// 跨行选中的出现次数与上下文同口径，不能出现"有上下文却显示 0 次"。
    #[test]
    fn hits_matches_the_same_key_as_context() {
        let log = TranscriptLog::new();
        log.push("灰度 范围 限定 华中");
        log.push("其他内容");
        log.push("灰度 范围 限定 华东");
        assert_eq!(log.hits("灰度 范围 限定 华中 我们 再 看"), 1);
        assert_eq!(log.hits("整段里没有的句子"), 0);
    }

    #[test]
    fn splits_chinese_and_ascii_sentences() {
        let parts = split_sentences("第一句。第二句！第三句? 第四句;");
        assert_eq!(parts, vec!["第一句", "第二句", "第三句", "第四句"]);
        assert!(split_sentences("   ").is_empty());
    }

    /// vosk 的中文按词带空格，展示前要合上；英文短语与中英混排的空格要留。
    #[test]
    fn tidies_cjk_spaces_but_keeps_english() {
        assert_eq!(
            tidy_cjk_spaces("学历 歧视 的 现象 仍 不 少见"),
            "学历歧视的现象仍不少见"
        );
        assert_eq!(
            tidy_cjk_spaces("通常 对应 英文 degree discrimination"),
            "通常对应英文 degree discrimination"
        );
        assert_eq!(tidy_cjk_spaces("SLA 指标 看 99.9%"), "SLA 指标看 99.9%");
        assert_eq!(tidy_cjk_spaces("  多  余  空  格  "), "多余空格");
        assert_eq!(tidy_cjk_spaces("中文 ， 标点"), "中文，标点");
    }
}
