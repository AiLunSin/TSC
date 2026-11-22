//! 判定内核的性能预算用例（计时只允许出现在测试目标）。

#[cfg(target_os = "windows")]
#[test]
fn thousand_pairs_under_200ms() {
    use meeting_copilot_domain::merge::{classify_equivalence, MergePolicy};

    const CI_BUDGET_MS: u128 = 500;

    let policy = MergePolicy::default();
    let base = "会议决定下周三开始灰度上线并完成监控告警演练，由张三负责回滚方案。";
    let other = "会议决定下周三开始灰度上线并完成监控告警演练，由李四负责回滚方案。";
    let started = std::time::Instant::now();
    let mut hits = 0u32;
    for _ in 0..1_000 {
        if classify_equivalence(base, other, &policy).is_some() {
            hits += 1;
        }
    }
    let elapsed_ms = started.elapsed().as_millis();
    println!("elapsed_ms={elapsed_ms} hits={hits} budget_ms={CI_BUDGET_MS}");
    assert!(
        elapsed_ms < CI_BUDGET_MS,
        "耗时 {elapsed_ms} ms 超出 CI 容差"
    );
}
