//! 超长正文的确定性切分（C-02）：2000 标量硬上限，句读优先，回退硬切。

use thiserror::Error;

use crate::model::TextRange;
use crate::vo::UtteranceText;

/// 一级句读：句末与换行。
pub const PRIMARY_BREAKS: &[char] = &['。', '！', '？', '；', '\n', '.', '!', '?', ';'];
/// 二级句读：句中停顿。
pub const SECONDARY_BREAKS: &[char] = &['，', '、', '：', ',', ':', ' '];

/// 切分后的单元。
#[derive(Debug, Clone, PartialEq)]
pub struct SplitChunk {
    pub index: u32,
    pub start_char: usize,
    pub end_char: usize,
    pub text: UtteranceText,
    pub span: TextRange,
}

/// 切分违规项；不并入 `ErrorCode` 冻结集。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SplitError {
    #[error("正文为空")]
    EmptyText,
    #[error("上限 {limit} 越界，允许 1..=2000")]
    LimitOutOfRange { limit: usize },
    #[error("区间 {span_ms} ms 不足以分配 {chunks} 段")]
    SpanTooShort { span_ms: u64, chunks: usize },
}

fn break_index(chars: &[char], start: usize, end: usize, breaks: &[char]) -> Option<usize> {
    (start + 1..=end)
        .rev()
        .find(|index| breaks.contains(&chars[index - 1]))
}

/// 在 `[start+1, end]` 内从右向左找一级断点，其次二级；均无则硬切到 `end`。
fn next_cut(chars: &[char], start: usize, end: usize) -> usize {
    if let Some(index) = break_index(chars, start, end, PRIMARY_BREAKS) {
        return index;
    }
    if let Some(index) = break_index(chars, start, end, SECONDARY_BREAKS) {
        return index;
    }
    end
}

/// 切分正文；返回的片段满足 I1~I6。
pub fn split_utterance(
    raw: &str,
    span: TextRange,
    limit: usize,
) -> Result<Vec<SplitChunk>, SplitError> {
    if limit == 0 || limit > 2_000 {
        return Err(SplitError::LimitOutOfRange { limit });
    }
    let chars: Vec<char> = raw.chars().collect();
    let total = chars.len();
    if total == 0 {
        return Err(SplitError::EmptyText);
    }

    let mut boundaries = Vec::new();
    let mut start = 0;
    while start < total {
        let end = (start + limit).min(total);
        let mut cut = next_cut(&chars, start, end);
        if cut <= start {
            cut = end;
        }
        boundaries.push((start, cut));
        start = cut;
    }

    let chunks = boundaries.len();
    let span_ms = span.end_ms.saturating_sub(span.start_ms);
    if span_ms < u64::try_from(chunks).unwrap_or(u64::MAX) {
        return Err(SplitError::SpanTooShort { span_ms, chunks });
    }

    let mut out = Vec::with_capacity(chunks);
    for (index, (start_char, end_char)) in boundaries.iter().enumerate() {
        let piece: String = chars[*start_char..*end_char].iter().collect();
        let piece_text = UtteranceText::new(piece).map_err(|_| SplitError::EmptyText)?;
        let offset_start = span.start_ms
            + span_ms * u64::try_from(*start_char).unwrap_or(u64::MAX)
                / u64::try_from(total).unwrap_or(1);
        let offset_end = if index + 1 == chunks {
            span.end_ms
        } else {
            span.start_ms
                + span_ms * u64::try_from(*end_char).unwrap_or(u64::MAX)
                    / u64::try_from(total).unwrap_or(1)
        };
        out.push(SplitChunk {
            index: u32::try_from(index).unwrap_or(u32::MAX),
            start_char: *start_char,
            end_char: *end_char,
            text: piece_text,
            span: TextRange {
                start_ms: offset_start,
                end_ms: offset_end,
            },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u64, end: u64) -> TextRange {
        TextRange {
            start_ms: start,
            end_ms: end,
        }
    }

    #[test]
    fn split_limits_are_enforced() {
        assert_eq!(
            split_utterance("abc", span(0, 100), 0),
            Err(SplitError::LimitOutOfRange { limit: 0 })
        );
        assert_eq!(
            split_utterance("abc", span(0, 100), 2_001),
            Err(SplitError::LimitOutOfRange { limit: 2_001 })
        );
        assert_eq!(
            split_utterance("", span(0, 100), 10),
            Err(SplitError::EmptyText)
        );
    }

    #[test]
    fn split_4001_chars_into_three_chunks() {
        let raw = "会".repeat(4_001);
        let chunks = split_utterance(&raw, span(0, 4_001), 2_000).expect("split");
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].start_char, 0);
        assert_eq!(chunks[2].end_char, 4_001);
    }

    #[test]
    fn split_without_punctuation_hard_cuts_at_limit() {
        let raw = "a".repeat(4_001);
        let chunks = split_utterance(&raw, span(0, 4_001), 2_000).expect("split");
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].end_char, 2_000);
        assert_eq!(chunks[1].end_char, 4_000);
    }

    #[test]
    fn split_is_reversible_and_exact_concat() {
        let raw = "第一句。第二句，第三句！剩余内容。".repeat(200);
        let chunks = split_utterance(&raw, span(0, 10_000), 2_000).expect("split");
        let joined: String = chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        assert_eq!(joined, raw);
        for chunk in &chunks {
            assert!(chunk.text.char_count() <= 2_000);
        }
    }

    #[test]
    fn split_spans_are_contiguous() {
        let raw = "会".repeat(4_001);
        let chunks = split_utterance(&raw, span(1_000, 5_001), 2_000).expect("split");
        assert_eq!(chunks[0].start_char, 0);
        for pair in chunks.windows(2) {
            assert_eq!(pair[1].start_char, pair[0].end_char);
        }
        assert_eq!(chunks.last().map(|chunk| chunk.end_char), Some(4_001));
    }

    #[test]
    fn split_timeline_is_proportional() {
        let raw = "会".repeat(4_000);
        let chunks = split_utterance(&raw, span(0, 4_000), 2_000).expect("split");
        assert_eq!(chunks[0].span.start_ms, 0);
        assert_eq!(chunks[0].span.end_ms, 2_000);
        assert_eq!(chunks[1].span.end_ms, 4_000);
        for chunk in &chunks {
            assert!(chunk.span.end_ms > chunk.span.start_ms);
        }
    }

    #[test]
    fn split_is_deterministic_across_100_runs() {
        let raw = "第一句。第二句，第三句！".repeat(100);
        let first = split_utterance(&raw, span(0, 9_000), 1_000).expect("split");
        for _ in 0..100 {
            assert_eq!(
                first,
                split_utterance(&raw, span(0, 9_000), 1_000).expect("split")
            );
        }
    }
}
