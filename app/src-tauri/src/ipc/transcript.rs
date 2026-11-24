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
/// 半句拼接后单行的上限：超时反复触发时不会把一整场会议并成一行。
const MAX_LINE_CHARS: usize = 400;
/// 拼接时允许丢弃的最大重叠长度：`Reset()` 后通常只重复 1~3 个词。
const MAX_OVERLAP: usize = 6;

/// 转写全文缓冲。
#[derive(Debug, Default)]
pub struct TranscriptLog {
    lines: Mutex<Vec<TranscriptLineDto>>,
}

impl TranscriptLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 稳定句入栈，返回这一行最终的完整文本（拼接后可能是整行）。
    ///
    /// `continues` 为真表示上一句是**超时截断的半句**：本句接到上一行末尾，而不是
    /// 另起一行。识别脚本在时长上限处强制收句，切出来的半句单独成行会让转写读起来
    /// 断断续续（`…社会仍然比较` / `落后很多` 各占一行）。拼接时去掉重复的重叠词：
    /// `Reset()` 后常有 1~3 个词被重新识别一次。
    pub fn push_stable(&self, text: &str, continues: bool) -> String {
        let text = tidy_cjk_spaces(text.trim());
        if text.is_empty() {
            return String::new();
        }
        let Ok(mut lines) = self.lines.lock() else {
            return text;
        };
        if continues {
            if let Some(last) = lines.last_mut() {
                let merged = join_fragments(&last.text, &text);
                // 反复超时会把一行越接越长；到上限就另起一行，别并成一整篇。
                if merged.chars().count() <= MAX_LINE_CHARS {
                    last.text.clone_from(&merged);
                    last.at_ms = now_ms();
                    return merged;
                }
            }
        }
        if lines.last().is_some_and(|last| last.text == text) {
            return text;
        }
        let index = lines.last().map_or(1, |last| last.index + 1);
        lines.push(TranscriptLineDto {
            index,
            text: text.clone(),
            at_ms: now_ms(),
        });
        if lines.len() > MAX_LINES {
            lines.remove(0);
        }
        text
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
            .filter(|line| compact(&line.text).contains(key.as_str()))
            .count()
    }
}

fn locate(lines: &[TranscriptLineDto], selection: &str) -> Option<usize> {
    let key = match_key(lines, selection)?;
    lines
        .iter()
        .position(|line| compact(&line.text).contains(key.as_str()))
}

/// 匹配键：整段 → 单句（长句优先）→ 连续词块（长块优先）。
///
/// 一律在**去空白**的形态上比较：转写行已经合过中文词间空格，而选区可能带空格
/// （跨行选中时前端用空格连接），两边直接 `contains` 会互相错过。
fn match_key(lines: &[TranscriptLineDto], selection: &str) -> Option<String> {
    let whole = compact(selection);
    if whole.is_empty() {
        return None;
    }
    if contains_any(lines, &whole) {
        return Some(whole);
    }
    let mut sentences: Vec<String> = split_sentences(selection)
        .into_iter()
        .map(compact)
        .filter(|sentence| sentence.chars().count() >= 2)
        .collect();
    sentences.sort_by_key(|sentence| std::cmp::Reverse(sentence.chars().count()));
    if let Some(hit) = sentences
        .into_iter()
        .find(|sentence| contains_any(lines, sentence))
    {
        return Some(hit);
    }
    let tokens: Vec<&str> = selection.split_whitespace().collect();
    let widest = tokens.len().min(8);
    for width in (2..=widest).rev() {
        for chunk in tokens.windows(width) {
            let key = compact(&chunk.join(" "));
            if key.chars().count() >= 2 && contains_any(lines, &key) {
                return Some(key);
            }
        }
    }
    None
}

fn contains_any(lines: &[TranscriptLineDto], needle: &str) -> bool {
    lines
        .iter()
        .any(|line| compact(&line.text).contains(needle))
}

/// 去掉所有空白：用于"与空格无关"的匹配。
fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
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

/// 单个字符是否"中文类"：汉字、中文标点、全角字符。
///
/// 用于两处：合并 vosk 的词间空格，以及拼接半句时判断要不要补空格。
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3000..=0x303F   // 中文标点（、。〈〉《》「」…）
        | 0x3400..=0x4DBF // 扩展 A
        | 0x4E00..=0x9FFF // 基本汉字
        | 0xF900..=0xFAFF // 兼容汉字
        | 0xFF01..=0xFF5E // 全角字符与标点
        | 0xFFE0..=0xFFE6 // 全角货币等符号
    )
}

/// 去掉**两个中文片段之间**的空格。
///
/// vosk 的中文结果是按词带空格的（`学历 歧视 的 现象`），直接展示像被切碎，
/// 抽关键词和选中查询也会被空格干扰。这里只在"上一个字和下一个字都是中文/中文标点"
/// 时丢掉空格：英文短语（`degree discrimination`）、中英混排（`SLA 指标`）保持原样。
pub fn tidy_cjk_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            pending_space = false;
            let both_cjk = out.chars().next_back().is_some_and(is_cjk) && is_cjk(ch);
            if !both_cjk {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

/// 把续句接到上一行末尾；两段之间的重复词只保留一份。
///
/// 超时收句后 `Reset()` 会丢掉解码上下文，下一段常把结尾几个词重新识别一次
/// （`…仍然比较` + `比较落后`）。这里找最长公共重叠并按字符剪掉。
fn join_fragments(previous: &str, next: &str) -> String {
    let prev: Vec<char> = previous.chars().collect();
    let upcoming: Vec<char> = next.chars().collect();
    let limit = prev.len().min(upcoming.len()).min(MAX_OVERLAP);
    let mut overlap = 0;
    for size in (1..=limit).rev() {
        if prev[prev.len() - size..] == upcoming[..size] {
            overlap = size;
            break;
        }
    }
    let tail = &upcoming[overlap..];
    let mut out = previous.to_owned();
    if let Some(first) = tail.first() {
        // 中英混排要留一个空格（`…看 SLA`），中文之间直接连写。
        let need_space = out.chars().next_back().is_some_and(|c| !is_cjk(c)) && !is_cjk(*first);
        if need_space && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.extend(tail.iter());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试里只关心"新起一行"的场景，省略 `continues` 参数。
    impl TranscriptLog {
        fn push_line(&self, text: &str) {
            self.push_stable(text, false);
        }
    }

    #[test]
    fn pushes_and_indexes_lines() {
        let log = TranscriptLog::new();
        log.push_line("第一句");
        log.push_line("第二句");
        let lines = log.snapshot();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].index, 1);
        assert_eq!(lines[1].index, 2);
        assert_eq!(lines[1].text, "第二句");
    }

    #[test]
    fn ignores_blank_and_repeated_lines() {
        let log = TranscriptLog::new();
        log.push_line("同一句");
        log.push_line("同一句");
        log.push_line("   ");
        assert_eq!(log.snapshot().len(), 1);
    }

    #[test]
    fn context_includes_neighbours() {
        let log = TranscriptLog::new();
        log.push_line("前面一句");
        log.push_line("这里提到了灰度发布");
        log.push_line("后面一句");
        let context = log.context_around("灰度");
        assert!(context.contains("前面一句"), "{context}");
        assert!(context.contains("灰度发布"), "{context}");
        assert!(context.contains("后面一句"), "{context}");
    }

    /// 跨行选中（前端把换行压成空格）必须仍能定位到位置。
    #[test]
    fn context_survives_cross_line_selection() {
        let log = TranscriptLog::new();
        log.push_line("第一行 灰度 范围 限定 华中");
        log.push_line("第二行 我们 先 看 数据");
        log.push_line("第三行 结论 是 通过");
        let context = log.context_for("灰度 范围 限定 华中 我们 先 看 数据");
        assert!(context.contains("灰度"), "{context}");
        assert!(context.contains("第二行"), "{context}");
    }

    /// 跨行选中的出现次数与上下文同口径，不能出现"有上下文却显示 0 次"。
    #[test]
    fn hits_matches_the_same_key_as_context() {
        let log = TranscriptLog::new();
        log.push_line("灰度 范围 限定 华中");
        log.push_line("其他内容");
        log.push_line("灰度 范围 限定 华东");
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

    /// 超时截断的半句要接回上一行，而不是另起一行。
    #[test]
    fn continues_previous_line_for_cut_fragment() {
        let log = TranscriptLog::new();
        log.push_stable("今天的社会仍然比较", false);
        let merged = log.push_stable("落后", true);
        assert_eq!(merged, "今天的社会仍然比较落后");
        let lines = log.snapshot();
        assert_eq!(lines.len(), 1, "拼接后仍是一行");
        assert_eq!(lines[0].text, "今天的社会仍然比较落后");
    }

    /// 不续句时照旧另起一行。
    #[test]
    fn fresh_fragment_starts_new_line() {
        let log = TranscriptLog::new();
        log.push_stable("第一句", false);
        log.push_stable("第二句", false);
        assert_eq!(log.snapshot().len(), 2);
    }

    /// `Reset()` 后重复识别的结尾词只能留一份。
    #[test]
    fn stitching_drops_repeated_overlap() {
        let log = TranscriptLog::new();
        log.push_stable("今天的社会仍然比较", false);
        assert_eq!(
            log.push_stable("比较落后很多", true),
            "今天的社会仍然比较落后很多"
        );
        assert_eq!(log.snapshot().len(), 1);
    }

    /// 中英混排拼接要留空格，中文之间直接连写。
    #[test]
    fn stitching_keeps_space_between_english_words() {
        let log = TranscriptLog::new();
        log.push_stable("我们看 SLA", false);
        assert_eq!(
            log.push_stable("99.9 的达成率", true),
            "我们看 SLA 99.9 的达成率"
        );
    }

    /// 一行接得太长就另起一行，避免整场会议并成一条。
    #[test]
    fn stitching_stops_at_line_cap() {
        let log = TranscriptLog::new();
        log.push_stable(&"甲".repeat(MAX_LINE_CHARS - 2), false);
        let merged = log.push_stable("乙乙乙乙", true);
        assert_eq!(merged, "乙乙乙乙", "超上限应新起一行");
        assert_eq!(log.snapshot().len(), 2);
    }
}
