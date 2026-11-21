//! `Metric::ALL` 契约冻结（AC-B2-OC-E10，跨包执行）。

use std::collections::BTreeSet;

use meeting_copilot_infrastructure::metrics::Metric;

/// AC-B2-OC-E10：10 项指标名称互不重复。
#[test]
fn metric_all_len_frozen() {
    assert_eq!(Metric::ALL.len(), 10);
    let names: BTreeSet<&'static str> = Metric::ALL.iter().map(|metric| metric.name()).collect();
    assert_eq!(names.len(), 10);
}
