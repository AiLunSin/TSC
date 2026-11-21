//! 三级等价判定：归一化相等 → SimHash 近邻 → 字符 3-gram 余弦（整数定点）。
//!
//! 明确排除编辑距离与前缀/后缀包含；全部整数运算，无浮点。

use std::cmp::Ordering;

use super::hash::{hamming_distance, simhash64};
use super::normalize::normalize_text;
use super::policy::MergePolicy;

/// 判定依据，三值封闭。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairBasis {
    Identical,
    NearHash { distance: u8 },
    Cosine { milli: u16 },
}

fn ngram_counts(text: &str, width: usize) -> Vec<(String, u32)> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < width {
        return Vec::new();
    }
    let mut grams: Vec<String> = (0..=chars.len() - width)
        .map(|start| chars[start..start + width].iter().collect())
        .collect();
    grams.sort_unstable();
    let mut out: Vec<(String, u32)> = Vec::new();
    for gram in grams {
        match out.last_mut() {
            Some(last) if last.0 == gram => last.1 += 1,
            _ => out.push((gram, 1)),
        }
    }
    out
}

fn isqrt(value: u128) -> u128 {
    if value == 0 {
        return 0;
    }
    let mut current = value;
    let mut next = current.div_ceil(2);
    while next < current {
        current = next;
        next = (current + value / current) / 2;
    }
    current
}

/// `1000 * Σmin(ca,cb) / isqrt(Σca² * Σcb²)`；空集返回 0，上限 1000。
pub fn ngram_cosine_milli(left: &str, right: &str) -> u16 {
    let a = ngram_counts(left, 3);
    let b = ngram_counts(right, 3);
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let mut i = 0;
    let mut j = 0;
    let mut dot: u128 = 0;
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                dot += u128::from(a[i].1) * u128::from(b[j].1);
                i += 1;
                j += 1;
            }
        }
    }
    let sum_a: u128 = a
        .iter()
        .map(|(_, count)| u128::from(*count) * u128::from(*count))
        .sum();
    let sum_b: u128 = b
        .iter()
        .map(|(_, count)| u128::from(*count) * u128::from(*count))
        .sum();
    let denominator = isqrt(sum_a * sum_b);
    if denominator == 0 {
        return 0;
    }
    let milli = dot.saturating_mul(1000) / denominator;
    u16::try_from(milli.min(1000)).unwrap_or(1000)
}

/// 三级判定；全部不命中返回 `None`。
pub fn classify_equivalence(left: &str, right: &str, policy: &MergePolicy) -> Option<PairBasis> {
    let a = normalize_text(left);
    let b = normalize_text(right);
    if a == b {
        return Some(PairBasis::Identical);
    }
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    if a_len == 0 || b_len == 0 {
        return None;
    }
    let a_len = u64::try_from(a_len).ok()?;
    let b_len = u64::try_from(b_len).ok()?;
    let ratio_ab = a_len.saturating_mul(1000) / b_len;
    let ratio_ba = b_len.saturating_mul(1000) / a_len;
    let min = u64::from(policy.length_ratio_min_milli);
    let max = u64::from(policy.length_ratio_max_milli);
    let ratio_ok = ratio_ab >= min && ratio_ab <= max && ratio_ba >= min && ratio_ba <= max;
    if ratio_ok {
        let distance = hamming_distance(simhash64(&a), simhash64(&b));
        if distance <= policy.simhash_max_distance {
            return Some(PairBasis::NearHash { distance });
        }
    }
    let min_chars = a_len.min(b_len);
    if min_chars > u64::from(policy.cosine_min_chars) {
        let milli = ngram_cosine_milli(&a, &b);
        if milli >= policy.cosine_min_milli {
            return Some(PairBasis::Cosine { milli });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    #[test]
    fn cosine_milli_is_integer_and_bounded() {
        assert_eq!(
            ngram_cosine_milli("确认下周灰度上线", "确认下周灰度上线"),
            1000
        );
        let partial = ngram_cosine_milli(
            "会议决定下周三开始灰度上线并完成监控演练",
            "会议决定下周三开始灰度上线并完成告警演练",
        );
        assert!(partial > 0 && partial < 1000);
    }

    #[test]
    fn cosine_edge_cases() {
        assert_eq!(ngram_cosine_milli("", "确认下周上线"), 0);
        assert_eq!(ngram_cosine_milli("确认下周上线", ""), 0);
        let long_a: String = "会议决定下周三开始灰度上线并完成监控告警演练".repeat(40);
        let long_b: String = "会议决定下周三开始灰度上线并完成监控告警演练".repeat(40);
        assert_eq!(ngram_cosine_milli(&long_a, &long_b), 1000);
        assert!(ngram_cosine_milli(&long_a, "另一段完全不同的文本") <= 1000);
    }

    #[test]
    fn classification_order_is_stable() {
        let policy = MergePolicy::default();
        assert_eq!(
            classify_equivalence("确认下周灰度上线。", "确认下周灰度上线", &policy),
            Some(PairBasis::Identical)
        );

        let mut relaxed = MergePolicy::default();
        relaxed.simhash_max_distance = 8;
        assert!(matches!(
            classify_equivalence(
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案",
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案确认",
                &relaxed
            ),
            Some(PairBasis::NearHash { .. })
        ));

        let mut strict = MergePolicy::default();
        strict.simhash_max_distance = 0;
        assert!(matches!(
            classify_equivalence(
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案确认",
                "会议决定下周三开始灰度上线并完成监控告警演练由张四负责回滚方案确认",
                &strict
            ),
            Some(PairBasis::Cosine { .. })
        ));
    }

    #[test]
    fn short_fragment_skips_cosine() {
        let mut policy = MergePolicy::default();
        policy.simhash_max_distance = 0;
        let eight = "确认下周灰度上线";
        let nine = "确认下周灰度上线了";
        let ten = "确认下周灰度上线了吗";
        assert_eq!(eight.chars().count(), 8);
        assert_eq!(nine.chars().count(), 9);
        assert_eq!(classify_equivalence(eight, nine, &policy), None);
        assert!(classify_equivalence(nine, ten, &policy).is_some());
    }

    #[test]
    fn pair_basis_is_closed() {
        assert_eq!(
            classify_equivalence(
                "确认下周灰度上线。",
                "确认下周灰度上线",
                &MergePolicy::default()
            ),
            Some(PairBasis::Identical)
        );
        let mut relaxed = MergePolicy::default();
        relaxed.simhash_max_distance = 8;
        assert!(matches!(
            classify_equivalence(
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案",
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案确认",
                &relaxed
            ),
            Some(PairBasis::NearHash { .. })
        ));
        let mut strict = MergePolicy::default();
        strict.simhash_max_distance = 0;
        assert!(matches!(
            classify_equivalence(
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案确认",
                "会议决定下周三开始灰度上线并完成监控告警演练由张四负责回滚方案确认",
                &strict
            ),
            Some(PairBasis::Cosine { .. })
        ));
    }

    #[test]

    fn length_ratio_bounds_are_exclusive() {
        let mut policy = MergePolicy::default();
        policy.simhash_max_distance = 8;
        let short = "确认下周灰度上线";
        let long = "确认下周灰度上线并完成监控告警演练与回滚方案确认";
        let basis = classify_equivalence(short, long, &policy);
        assert!(!matches!(basis, Some(PairBasis::NearHash { .. })));
    }

    #[test]

    fn thousand_runs_are_bitwise_identical() {
        let policy = MergePolicy::default();
        let samples = [
            ("确认下周灰度上线。", "确认下周灰度上线"),
            ("确认下周灰度上线", "确认下周灰度上线了"),
            (
                "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案确认",
                "会议决定下周三开始灰度上线并完成监控告警演练由张四负责回滚方案确认",
            ),
            ("完全不同的甲", "完全不同的乙"),
        ];
        for (left, right) in samples {
            let first = classify_equivalence(left, right, &policy);
            for _ in 0..1_000 {
                assert_eq!(first, classify_equivalence(left, right, &policy));
            }
        }
    }
}
