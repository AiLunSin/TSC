//! 28.2 静音判定：按重叠比例给出 丢弃 / 部分重叠 / 无交集 三态。

/// 静音判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SilenceVerdict {
    None,
    Partial,
    Dropped,
}

/// `overlap_ms / span_ms >= threshold_milli` 时整体丢弃；否则部分重叠放行。
pub fn classify(overlap_ms: u64, span_ms: u64, threshold_milli: u16) -> SilenceVerdict {
    if overlap_ms == 0 {
        return SilenceVerdict::None;
    }
    let ratio = overlap_ms.saturating_mul(1000) / span_ms.max(1);
    if ratio >= u64::from(threshold_milli) {
        SilenceVerdict::Dropped
    } else {
        SilenceVerdict::Partial
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_verdict_is_three_state() {
        assert_eq!(classify(0, 1_000, 800), SilenceVerdict::None);
        assert_eq!(classify(500, 1_000, 800), SilenceVerdict::Partial);
        assert_eq!(classify(800, 1_000, 800), SilenceVerdict::Dropped);
        assert_eq!(classify(1_000, 1_000, 800), SilenceVerdict::Dropped);
    }

    #[test]
    fn silence_ratio_handles_zero_span() {
        assert_eq!(classify(1, 0, 800), SilenceVerdict::Dropped);
    }
}
