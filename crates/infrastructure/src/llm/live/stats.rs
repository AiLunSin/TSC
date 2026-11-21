//! 联调统计口径（nearest-rank、降级排除、六类失败分布）。

use meeting_copilot_domain::model::llm::{FailureClass, LlmTask};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sample {
    pub seq: u32,
    pub task: LlmTask,
    pub attempt: u8,
    pub first_token_ms: Option<u32>,
    pub total_ms: u64,
    pub outcome: String,
    pub failure_class: Option<FailureClass>,
    pub http_status: Option<u16>,
    pub degraded: bool,
    pub excluded: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskStats {
    pub task: LlmTask,
    pub n: usize,
    pub valid: usize,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub native_success_rate: Option<f64>,
    pub degraded_count: usize,
    pub excluded_count: usize,
    pub failures: [usize; 6],
    pub unclassified: usize,
}

impl TaskStats {
    pub fn sample_insufficient(&self) -> bool {
        self.n < 30
    }

    pub fn success_rate_insufficient(&self) -> bool {
        self.n < 10
    }
}

/// nearest-rank：升序第 ceil(q × n) 个值。
pub fn percentile(values: &[u64], q: f64) -> Option<u64> {
    if values.is_empty() || !(0.0..=1.0).contains(&q) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((q * sorted.len() as f64).ceil() as usize).max(1);
    sorted.get(rank - 1).copied()
}

pub fn summarize(samples: &[Sample], task: LlmTask) -> TaskStats {
    let task_samples: Vec<&Sample> = samples
        .iter()
        .filter(|sample| sample.task == task)
        .collect();
    let valid: Vec<&&Sample> = task_samples
        .iter()
        .filter(|sample| !sample.degraded && sample.excluded.is_none())
        .collect();
    let durations: Vec<u64> = valid.iter().map(|sample| sample.total_ms).collect();
    let native_ok = task_samples
        .iter()
        .filter(|sample| !sample.degraded && sample.excluded.is_none())
        .count();
    let degraded_count = task_samples.iter().filter(|sample| sample.degraded).count();
    let excluded_count = task_samples
        .iter()
        .filter(|sample| sample.excluded.is_some())
        .count();
    let mut failures = [0_usize; 6];
    let mut unclassified = 0_usize;
    for sample in &task_samples {
        match sample.failure_class {
            Some(class) => {
                let index = FailureClass::ALL
                    .iter()
                    .position(|item| *item == class)
                    .unwrap_or(5);
                failures[index] += 1;
            }
            None if sample.degraded => unclassified += 1,
            None => {}
        }
    }
    let native_success_rate = if task_samples.len() < 10 {
        None
    } else {
        Some(native_ok as f64 / task_samples.len() as f64)
    };
    let p50 = percentile(&durations, 0.50);
    let p95 = if task_samples.len() < 30 {
        None
    } else {
        percentile(&durations, 0.95)
    };
    TaskStats {
        task,
        n: task_samples.len(),
        valid: valid.len(),
        p50_ms: p50,
        p95_ms: p95,
        native_success_rate,
        degraded_count,
        excluded_count,
        failures,
        unclassified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(seq: u32, total_ms: u64, degraded: bool, failure: Option<FailureClass>) -> Sample {
        Sample {
            seq,
            task: LlmTask::InsightExtract,
            attempt: 1,
            first_token_ms: Some(u32::try_from(total_ms).unwrap_or(u32::MAX)),
            total_ms,
            outcome: if degraded {
                "degraded".into()
            } else {
                "ok".into()
            },
            failure_class: failure,
            http_status: None,
            degraded,
            excluded: None,
        }
    }

    #[test]
    fn nearest_rank_boundaries() {
        assert_eq!(percentile(&[42], 0.5), Some(42));
        assert_eq!(percentile(&[1, 2], 0.5), Some(1));
        assert_eq!(percentile(&[1, 2], 0.95), Some(2));
        let values: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&values, 0.95), Some(95));
    }

    #[test]
    fn degraded_samples_are_excluded_from_latency() {
        let samples = vec![
            sample(1, 10, false, None),
            sample(2, 1_000, true, Some(FailureClass::Timeout)),
            sample(3, 20, false, None),
        ];
        let stats = summarize(&samples, LlmTask::InsightExtract);
        assert_eq!(stats.valid, 2);
        assert_eq!(stats.degraded_count, 1);
        assert_eq!(stats.p50_ms, Some(10));
    }

    #[test]
    fn insufficient_samples_do_not_emit_p95() {
        let short: Vec<Sample> = (1..=29).map(|seq| sample(seq, 10, false, None)).collect();
        let stats = summarize(&short, LlmTask::InsightExtract);
        assert!(stats.sample_insufficient());
        assert_eq!(stats.p95_ms, None);
        assert_eq!(stats.p50_ms, Some(10));

        let thirty: Vec<Sample> = (1..=30).map(|seq| sample(seq, 10, false, None)).collect();
        let stats = summarize(&thirty, LlmTask::InsightExtract);
        assert!(!stats.sample_insufficient());
        assert_eq!(stats.p95_ms, Some(10));

        let nine: Vec<Sample> = (1..=9).map(|seq| sample(seq, 10, false, None)).collect();
        let stats = summarize(&nine, LlmTask::InsightExtract);
        assert!(stats.success_rate_insufficient());
        assert_eq!(stats.native_success_rate, None);
    }

    #[test]
    fn failure_distribution_covers_six_and_unclassified() {
        let mut samples = Vec::new();
        for (index, class) in FailureClass::ALL.iter().enumerate() {
            samples.push(sample(
                u32::try_from(index).expect("index") + 1,
                100,
                true,
                Some(*class),
            ));
        }
        samples.push(sample(99, 100, true, None));
        let stats = summarize(&samples, LlmTask::InsightExtract);
        assert_eq!(stats.failures.iter().sum::<usize>(), 6);
        assert_eq!(stats.unclassified, 1);
    }

    #[test]
    fn success_rate_excludes_degraded_from_numerator() {
        let mut samples: Vec<Sample> = (1..=10).map(|seq| sample(seq, 10, false, None)).collect();
        samples.push(sample(11, 10, true, Some(FailureClass::Timeout)));
        let stats = summarize(&samples, LlmTask::InsightExtract);
        assert!((stats.native_success_rate.expect("rate") - 10.0 / 11.0).abs() < 1e-9);
    }
}
