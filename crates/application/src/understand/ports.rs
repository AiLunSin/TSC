//! 理解层端口与只读信号。

use meeting_copilot_domain::model::session::TextRange;

/// 理解层时钟：只暴露毫秒，测试用虚拟时钟。
pub trait UnderstandClock: Send + Sync {
    fn now_millis(&self) -> u64;
}

/// feature 开关（默认关闭由调用方决定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderstandFeature {
    Disabled,
    Enabled,
}

/// 静音区间信号：只读消费，不实现 VAD。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SilenceSignal {
    intervals: Vec<TextRange>,
}

impl SilenceSignal {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn from_intervals(intervals: Vec<TextRange>) -> Self {
        Self { intervals }
    }

    pub fn interval_count(&self) -> usize {
        self.intervals.len()
    }

    /// 与静音区间的重叠毫秒数。
    pub fn overlap_ms(&self, span: TextRange) -> u64 {
        self.intervals
            .iter()
            .map(|interval| {
                let begin = interval.start_ms.max(span.start_ms);
                let finish = interval.end_ms.min(span.end_ms);
                finish.saturating_sub(begin)
            })
            .sum()
    }

    /// 片段是否与任一静音区间重叠。
    pub fn overlaps(&self, span: TextRange) -> bool {
        self.intervals
            .iter()
            .any(|interval| interval.start_ms < span.end_ms && span.start_ms < interval.end_ms)
    }
}

/// 术语快照：去空白、去重、字节序排序，最多 64 项。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LexiconView {
    names: Vec<String>,
}

impl LexiconView {
    pub fn from_names(names: Vec<String>) -> Self {
        let mut deduped: Vec<String> = names
            .into_iter()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect();
        deduped.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        deduped.dedup();
        deduped.truncate(64);
        Self { names: deduped }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }
}
