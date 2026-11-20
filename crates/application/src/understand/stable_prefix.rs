//! 稳定前缀策略（SPEC 4.4-4、5.2.1-8；DESIGN 1.2.3）。
//!
//! 流式 ASR 的 partial 会抖动。只有同一结果连续出现两次才提交；
//! 已提交内容永不回退，尾部修正按拒绝计数，供误报率统计。

/// 同一 partial 连续出现的提交阈值。
pub const STABLE_REPEATS: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StablePrefix {
    committed: String,
    candidate: String,
    repeats: u8,
    rejected_revisions: u32,
}

impl StablePrefix {
    pub fn new() -> Self {
        Self::default()
    }

    /// 输入一次 partial，返回本次新增的稳定文本。
    pub fn observe(&mut self, partial: &str) -> Option<String> {
        let normalized = partial.trim_end();
        if normalized.is_empty() {
            return None;
        }

        if normalized == self.candidate {
            self.repeats = self.repeats.saturating_add(1);
        } else {
            self.candidate = normalized.to_owned();
            self.repeats = 1;
            return None;
        }

        if self.repeats != STABLE_REPEATS {
            return None;
        }

        match normalized.strip_prefix(&self.committed) {
            Some(delta) if !delta.is_empty() => {
                self.committed.push_str(delta);
                Some(delta.to_owned())
            }
            Some(_) => None,
            None => {
                // 已提交内容不回退：尾部修正或整句重写只计数，不提交。
                self.rejected_revisions = self.rejected_revisions.saturating_add(1);
                None
            }
        }
    }

    pub fn committed(&self) -> &str {
        &self.committed
    }

    pub fn rejected_revisions(&self) -> u32 {
        self.rejected_revisions
    }

    /// 新句/新段开始时清空，已落库内容由调用方负责。
    pub fn reset(&mut self) {
        self.committed.clear();
        self.candidate.clear();
        self.repeats = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(filter: &mut StablePrefix, partials: &[&str]) -> Vec<Option<String>> {
        partials.iter().map(|p| filter.observe(p)).collect()
    }

    #[test]
    fn character_by_character_append() {
        let mut filter = StablePrefix::new();
        let updates = feed(&mut filter, &["你", "你好", "你好", "你好吗", "你好吗"]);

        assert_eq!(
            updates,
            vec![
                None,
                None,
                Some("你好".to_owned()),
                None,
                Some("吗".to_owned())
            ]
        );
        assert_eq!(filter.committed(), "你好吗");
        assert_eq!(filter.rejected_revisions(), 0);
    }

    #[test]
    fn full_sentence_replacement_after_reset() {
        let mut filter = StablePrefix::new();
        feed(&mut filter, &["今天开会", "今天开会"]);
        assert_eq!(filter.committed(), "今天开会");

        filter.reset();
        let updates = feed(&mut filter, &["明天评审", "明天评审"]);
        assert_eq!(updates, vec![None, Some("明天评审".to_owned())]);
        assert_eq!(filter.committed(), "明天评审");
    }

    #[test]
    fn tail_correction_is_rejected_without_rollback() {
        let mut filter = StablePrefix::new();
        feed(&mut filter, &["今天开会", "今天开会"]);
        let updates = feed(&mut filter, &["今日开会", "今日开会"]);

        assert_eq!(updates, vec![None, None]);
        assert_eq!(filter.committed(), "今天开会", "已提交内容不得回退");
        assert_eq!(filter.rejected_revisions(), 1);
    }

    #[test]
    fn repeated_identical_partial_is_not_committed_twice() {
        let mut filter = StablePrefix::new();
        feed(&mut filter, &["确认上线", "确认上线"]);
        assert_eq!(filter.committed(), "确认上线");

        filter.observe("确认上线");
        filter.observe("确认上线");
        assert_eq!(filter.committed(), "确认上线");
    }

    #[test]
    fn empty_or_whitespace_is_ignored() {
        let mut filter = StablePrefix::new();
        assert_eq!(filter.observe(""), None);
        assert_eq!(filter.observe("   "), None);
        assert_eq!(filter.committed(), "");
    }
}
