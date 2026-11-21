//! 滚动摘要的消费式窗口与每会话状态位。

use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::rolling_summary::{SummaryStatus, SummaryVersionPair};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::UtteranceRef;

/// 增量窗口：`take` 即清空，不可二次取用。
#[derive(Debug, Clone, Default)]
pub struct SummaryWindow {
    pending: Vec<(UtteranceRef, TextRange)>,
    opened_at: u64,
}

impl SummaryWindow {
    pub fn accumulate(&mut self, reference: UtteranceRef, span: TextRange, now_ms: u64) {
        if self.pending.is_empty() {
            self.opened_at = now_ms;
        }
        self.pending.push((reference, span));
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn elapsed_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.opened_at)
    }

    /// 20 条或 30 s（先到者）；空窗口不触发。
    pub fn should_trigger(&self, policy: &InsightPolicy, now_ms: u64) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        self.pending.len() >= usize::from(policy.summary_utterance_count)
            || self.elapsed_ms(now_ms) >= policy.summary_interval_ms
    }

    /// 取走全部增量并清空；区间稳定排序后返回。
    pub fn take(&mut self) -> Vec<(UtteranceRef, TextRange)> {
        let mut delta = std::mem::take(&mut self.pending);
        self.opened_at = 0;
        delta.sort_by_key(|(_, span)| (span.start_ms, span.end_ms));
        delta
    }
}

/// 覆盖区间：排序后首段起、末段止。
pub fn covered_span(delta: &[(UtteranceRef, TextRange)]) -> Option<TextRange> {
    let first = delta.first()?;
    let last = delta.last()?;
    Some(TextRange {
        start_ms: first.1.start_ms,
        end_ms: last.1.end_ms,
    })
}

/// 每会话摘要状态：同会话串行，不同会话互不阻塞。
#[derive(Debug, Clone, Default)]
pub struct SessionSummaryState {
    pub pair: Option<SummaryVersionPair>,
    pub inflight: bool,
    pub status: Option<SummaryStatus>,
}

impl SessionSummaryState {
    pub fn begin_inflight(&mut self) -> bool {
        if self.inflight {
            return false;
        }
        self.inflight = true;
        true
    }

    pub fn finish_inflight(&mut self, status: SummaryStatus) {
        self.inflight = false;
        self.status = Some(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::vo::UtteranceRef;

    fn reference(name: &str) -> UtteranceRef {
        UtteranceRef::new(name).expect("ref")
    }

    fn span(start: u64, end: u64) -> TextRange {
        TextRange {
            start_ms: start,
            end_ms: end,
        }
    }

    fn policy() -> InsightPolicy {
        InsightPolicy::default()
    }

    #[test]
    fn window_take_is_consuming() {
        let mut window = SummaryWindow::default();
        window.accumulate(reference("u1"), span(0, 1_000), 0);
        let first = window.take();
        assert_eq!(first.len(), 1);
        assert!(window.take().is_empty());
        assert_eq!(window.pending_len(), 0);
    }

    #[test]
    fn refresh_triggers_at_twenty_utterances() {
        let mut window = SummaryWindow::default();
        for index in 0..20 {
            window.accumulate(reference(&format!("u{index}")), span(index, index + 1), 0);
        }
        assert!(window.should_trigger(&policy(), 0));
    }

    #[test]
    fn nineteen_utterances_do_not_trigger() {
        let mut window = SummaryWindow::default();
        for index in 0..19 {
            window.accumulate(reference(&format!("u{index}")), span(index, index + 1), 0);
        }
        assert!(!window.should_trigger(&policy(), 0));
    }

    #[test]
    fn refresh_triggers_at_thirty_seconds_once() {
        let mut window = SummaryWindow::default();
        window.accumulate(reference("u1"), span(0, 1_000), 0);
        assert!(!window.should_trigger(&policy(), 29_999));
        assert!(window.should_trigger(&policy(), 30_000));
        let _ = window.take();
        assert!(!window.should_trigger(&policy(), 30_000));
    }

    #[test]
    fn covered_span_equals_delta_window() {
        let mut window = SummaryWindow::default();
        window.accumulate(reference("u2"), span(2_000, 3_000), 0);
        window.accumulate(reference("u1"), span(0, 1_000), 0);
        let delta = window.take();
        assert_eq!(covered_span(&delta), Some(span(0, 3_000)));
        assert_eq!(delta[0].1.start_ms, 0);
    }

    #[test]
    fn inflight_is_mutually_exclusive() {
        let mut state = SessionSummaryState::default();
        assert!(state.begin_inflight());
        assert!(!state.begin_inflight());
        state.finish_inflight(SummaryStatus::Ok);
        assert!(state.begin_inflight());
    }
}
