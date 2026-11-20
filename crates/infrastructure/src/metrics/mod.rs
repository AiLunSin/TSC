//! 本地度量（SPEC 4.4-1/4-4、DESIGN 2.6.6）。原文一律不得进入指标。

use std::collections::BTreeMap;

/// 10 项必填指标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metric {
    InsightLatency,
    SuggestionLatencyP95,
    SuggestionLatencyP99,
    AutoTriggerRate,
    FalseTriggerRate,
    TranscriptionFailureRate,
    ProviderFailureRate,
    SubtitleFailureRateByPlatform,
    AdoptionRate,
    ActiveSessions,
}

impl Metric {
    pub const ALL: [Self; 10] = [
        Self::InsightLatency,
        Self::SuggestionLatencyP95,
        Self::SuggestionLatencyP99,
        Self::AutoTriggerRate,
        Self::FalseTriggerRate,
        Self::TranscriptionFailureRate,
        Self::ProviderFailureRate,
        Self::SubtitleFailureRateByPlatform,
        Self::AdoptionRate,
        Self::ActiveSessions,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::InsightLatency => "insight_latency_ms",
            Self::SuggestionLatencyP95 => "suggestion_latency_p95_ms",
            Self::SuggestionLatencyP99 => "suggestion_latency_p99_ms",
            Self::AutoTriggerRate => "auto_trigger_rate",
            Self::FalseTriggerRate => "false_trigger_rate",
            Self::TranscriptionFailureRate => "transcription_failure_rate",
            Self::ProviderFailureRate => "provider_failure_rate",
            Self::SubtitleFailureRateByPlatform => "subtitle_failure_rate_by_platform",
            Self::AdoptionRate => "adoption_rate",
            Self::ActiveSessions => "active_sessions",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub metric: Metric,
    pub value: f64,
    pub dimension: Option<String>,
}

#[derive(Debug, Default)]
pub struct MetricsStore {
    points: Vec<MetricPoint>,
}

impl MetricsStore {
    pub fn record(&mut self, point: MetricPoint) {
        self.points.push(point);
    }

    pub fn values(&self, metric: Metric) -> Vec<f64> {
        self.points
            .iter()
            .filter(|point| point.metric == metric)
            .map(|point| point.value)
            .collect()
    }

    pub fn percentile(&self, metric: Metric, percentile: f64) -> Option<f64> {
        let mut values = self.values(metric);
        if values.is_empty() {
            return None;
        }
        values.sort_by(f64::total_cmp);
        let rank = ((percentile / 100.0) * f64::from(u32::try_from(values.len()).ok()?))
            .ceil()
            .max(1.0);
        let index = usize::try_from(rank as u64).ok()?.saturating_sub(1);
        values.get(index.min(values.len() - 1)).copied()
    }

    pub fn aggregate_by_dimension(&self, metric: Metric) -> BTreeMap<String, f64> {
        let mut sums: BTreeMap<String, (f64, u32)> = BTreeMap::new();
        for point in self.points.iter().filter(|p| p.metric == metric) {
            let key = point.dimension.clone().unwrap_or_else(|| "all".to_owned());
            let entry = sums.entry(key).or_insert((0.0, 0));
            entry.0 += point.value;
            entry.1 += 1;
        }
        sums.into_iter()
            .map(|(key, (sum, count))| {
                let count = f64::from(count.max(1));
                (key, sum / count)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ten_metrics_present() {
        assert_eq!(Metric::ALL.len(), 10);
        let names: std::collections::HashSet<_> = Metric::ALL.iter().map(|m| m.name()).collect();
        assert_eq!(names.len(), 10);
    }

    #[test]
    fn p95_computation() {
        let mut store = MetricsStore::default();
        for value in 1..=100 {
            store.record(MetricPoint {
                metric: Metric::InsightLatency,
                value: f64::from(value),
                dimension: None,
            });
        }
        assert_eq!(store.percentile(Metric::InsightLatency, 95.0), Some(95.0));
    }

    #[test]
    fn dimension_aggregation() {
        let mut store = MetricsStore::default();
        for (platform, value) in [("zoom", 100.0), ("zoom", 200.0), ("feishu", 50.0)] {
            store.record(MetricPoint {
                metric: Metric::SubtitleFailureRateByPlatform,
                value,
                dimension: Some(platform.to_owned()),
            });
        }
        let grouped = store.aggregate_by_dimension(Metric::SubtitleFailureRateByPlatform);
        assert_eq!(grouped.get("zoom"), Some(&150.0));
        assert_eq!(grouped.get("feishu"), Some(&50.0));
    }
}
