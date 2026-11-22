//! 28.4 低置信分流：推测性 / 仅保存 / 可提取 三态。

/// 置信度分流结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceVerdict {
    Speculative,
    StoredOnly,
    Admitted,
}

/// 固定推测性边界 350‰；低于 `extract_min_milli` 仅保存。
pub fn classify(value_milli: u16, extract_min_milli: u16) -> ConfidenceVerdict {
    if value_milli < 350 {
        ConfidenceVerdict::Speculative
    } else if value_milli < extract_min_milli {
        ConfidenceVerdict::StoredOnly
    } else {
        ConfidenceVerdict::Admitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_verdict_boundaries() {
        assert_eq!(classify(349, 500), ConfidenceVerdict::Speculative);
        assert_eq!(classify(350, 500), ConfidenceVerdict::StoredOnly);
        assert_eq!(classify(499, 500), ConfidenceVerdict::StoredOnly);
        assert_eq!(classify(500, 500), ConfidenceVerdict::Admitted);
        assert_eq!(classify(900, 500), ConfidenceVerdict::Admitted);
    }
}
