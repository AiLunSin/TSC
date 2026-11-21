//! 去重判定依据：只保留 `Identical` 与带毫值的 `Cosine`。

/// 去重依据。不复用 `merge::PairBasis`（后者无毫值，会让 0.87 误合并）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupeBasis {
    Identical,
    Cosine { milli: u16 },
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_basis_is_closed() {
        let values = [DedupeBasis::Identical, DedupeBasis::Cosine { milli: 880 }];
        for value in values {
            match value {
                DedupeBasis::Identical | DedupeBasis::Cosine { .. } => {}
            }
        }
        assert_eq!(
            DedupeBasis::Cosine { milli: 880 },
            DedupeBasis::Cosine { milli: 880 }
        );
    }
}
