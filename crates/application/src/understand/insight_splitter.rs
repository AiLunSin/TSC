//! 要点正文 60 字强制拆分（复用 C-02 切分，不做 UI 截断）。

use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance_text::{split_utterance, SplitError};
use meeting_copilot_domain::vo::ShortText;

/// 每段正文与对应区间；拼接逐字等于原文。
pub fn split_insight_text(
    raw: &str,
    span: TextRange,
) -> Result<Vec<(ShortText, TextRange)>, SplitError> {
    let chunks = split_utterance(raw, span, ShortText::LIMIT)?;
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let text = ShortText::new(chunk.text.as_str()).map_err(|_| SplitError::EmptyText)?;
        out.push((text, chunk.span));
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
    fn overlong_insight_is_split() {
        let raw = "会".repeat(150);
        let parts = split_insight_text(&raw, span(0, 150)).expect("split");
        assert!(parts.len() >= 3);
        for (text, _) in &parts {
            assert!(text.char_count() <= ShortText::LIMIT);
        }
    }

    #[test]
    fn split_text_is_lossless() {
        let raw = "确认下周灰度上线并完成监控告警演练，由张三负责回滚方案。".repeat(6);
        let parts = split_insight_text(&raw, span(0, 1_000)).expect("split");
        let joined: String = parts.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(joined, raw);
    }

    #[test]
    fn short_text_is_not_split() {
        let parts = split_insight_text("确认下周上线", span(0, 1_000)).expect("split");
        assert_eq!(parts.len(), 1);
    }
}
