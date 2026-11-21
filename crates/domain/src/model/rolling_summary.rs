//! 滚动摘要的版本对、诊断状态与解析产物。

use thiserror::Error;

use crate::model::insight::{Entity, RollingSummary, Topic};

/// 当前版与上一版；保留上限固定 2。
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryVersionPair {
    pub current: RollingSummary,
    pub previous: Option<RollingSummary>,
}

/// 最近一次刷新结论（状态，不是次数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryStatus {
    Ok,
    Timeout,
    GatewayUnavailable,
    SchemaViolation,
    PublishBlocked,
    OutOfOrder,
}

/// 从摘要正文解析出的主题与实体。
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryParse {
    pub topics: Vec<Topic>,
    pub entities: Vec<Entity>,
}

/// 版本违规项；不并入 `ErrorCode` 冻结集。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SummaryVersionError {
    #[error("版本必须为 {expected}，实际 {actual}")]
    VersionNotMonotonic { expected: u32, actual: u32 },
    #[error("覆盖区间起点回退")]
    CoveredRegressed,
}

impl SummaryVersionPair {
    pub fn new(current: RollingSummary) -> Self {
        Self {
            current,
            previous: None,
        }
    }

    /// 版本严格单调、覆盖起点不回退；发布后旧版进入 `previous`。
    pub fn publish(&mut self, next: RollingSummary) -> Result<(), SummaryVersionError> {
        if next.version != self.current.version.saturating_add(1) {
            return Err(SummaryVersionError::VersionNotMonotonic {
                expected: self.current.version.saturating_add(1),
                actual: next.version,
            });
        }
        if next.covered.start_ms < self.current.covered.start_ms {
            return Err(SummaryVersionError::CoveredRegressed);
        }
        self.previous = Some(self.current.clone());
        self.current = next;
        Ok(())
    }

    /// 保留版数：1 或 2。
    pub fn retained_versions(&self) -> u8 {
        if self.previous.is_some() {
            2
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::session::TextRange;

    fn summary(version: u32, start: u64, end: u64) -> RollingSummary {
        RollingSummary {
            version,
            covered: TextRange {
                start_ms: start,
                end_ms: end,
            },
            topics: vec![Topic {
                label: "灰度".into(),
                weight: 1.0,
            }],
            entities: vec![Entity {
                name: "张三".into(),
                normalized: "张三".into(),
            }],
        }
    }

    #[test]
    fn summary_version_pair_keeps_two_versions() {
        let mut pair = SummaryVersionPair::new(summary(1, 0, 1_000));
        assert_eq!(pair.retained_versions(), 1);
        pair.publish(summary(2, 0, 2_000)).expect("publish");
        assert_eq!(pair.retained_versions(), 2);
        assert_eq!(pair.current.version, 2);
        assert_eq!(
            pair.previous.as_ref().map(|previous| previous.version),
            Some(1)
        );
    }

    #[test]
    fn summary_version_must_be_monotonic() {
        let mut pair = SummaryVersionPair::new(summary(1, 0, 1_000));
        assert_eq!(
            pair.publish(summary(3, 0, 2_000)),
            Err(SummaryVersionError::VersionNotMonotonic {
                expected: 2,
                actual: 3
            })
        );
    }

    #[test]
    fn covered_start_must_not_regress() {
        let mut pair = SummaryVersionPair::new(summary(1, 1_000, 2_000));
        assert_eq!(
            pair.publish(summary(2, 500, 3_000)),
            Err(SummaryVersionError::CoveredRegressed)
        );
    }
}
