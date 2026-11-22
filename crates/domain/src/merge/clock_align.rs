//! 24.1 音频时钟统一：把音频流相对区间对齐到会话单调时钟。

use crate::model::session::TextRange;

/// 会话时钟与音频流起点之间的偏移（毫秒，允许负值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockOffset {
    pub millis: i64,
}

impl ClockOffset {
    /// `session_start - audio_start`：音频时刻 + 偏移 = 会话时刻。
    pub fn between(session_start_ms: u64, audio_start_ms: u64) -> Option<Self> {
        let session = i64::try_from(session_start_ms).ok()?;
        let audio = i64::try_from(audio_start_ms).ok()?;
        Some(Self {
            millis: session.checked_sub(audio)?,
        })
    }

    /// 对齐后的会话区间；越界或退化区间返回 `None`。
    pub fn align(&self, audio_span: TextRange) -> Option<TextRange> {
        let start = i64::try_from(audio_span.start_ms)
            .ok()?
            .checked_add(self.millis)?;
        let end = i64::try_from(audio_span.end_ms)
            .ok()?
            .checked_add(self.millis)?;
        if start < 0 || end <= start {
            return None;
        }
        Some(TextRange {
            start_ms: u64::try_from(start).ok()?,
            end_ms: u64::try_from(end).ok()?,
        })
    }
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
    fn positive_offset_aligns_audio_to_session() {
        let offset = ClockOffset::between(10_000, 1_000).expect("offset");
        assert_eq!(offset.millis, 9_000);
        assert_eq!(offset.align(span(1_000, 2_000)), Some(span(10_000, 11_000)));
    }

    #[test]
    fn negative_offset_is_rejected_when_out_of_range() {
        let offset = ClockOffset::between(0, 1_000).expect("offset");
        assert_eq!(offset.millis, -1_000);
        assert_eq!(offset.align(span(500, 1_500)), None);
    }

    #[test]
    fn alignment_preserves_duration() {
        let offset = ClockOffset::between(5_000, 5_000).expect("offset");
        let aligned = offset.align(span(0, 1_500)).expect("aligned");
        assert_eq!(
            aligned.end_ms.saturating_sub(aligned.start_ms),
            1_500,
            "对齐不得改变区间长度"
        );
    }
}
