//! G2 相关性：整数加权与整数衰减，无浮点、无 IO。

use meeting_copilot_domain::model::gate::{GatePolicy, RelevanceScore};

use crate::gate::input::RelevanceInput;

pub(crate) const WEIGHT_SEMANTIC: u32 = 400;
pub(crate) const WEIGHT_ENTITY: u32 = 250;
pub(crate) const WEIGHT_GAIN: u32 = 200;
pub(crate) const WEIGHT_TRUST: u32 = 150;

fn clamp_milli(value: u16) -> u16 {
    value.min(1000)
}

/// `round(1000 × exp(-Δt/180000))` 的整数级数近似；Δt 上限 180 s。
pub fn decay_milli(delta_ms: u64) -> u16 {
    let x = i64::try_from(delta_ms.min(180_000))
        .unwrap_or(180_000)
        .saturating_mul(1000)
        / 180_000;
    let x2 = x.saturating_mul(x) / 2_000;
    let x3 = x.saturating_mul(x).saturating_mul(x) / 6_000_000;
    let x4 = x.saturating_mul(x).saturating_mul(x).saturating_mul(x) / 24_000_000_000;
    let x5 = x
        .saturating_mul(x)
        .saturating_mul(x)
        .saturating_mul(x)
        .saturating_mul(x)
        / 120_000_000_000_000;
    let value = 1000 - x + x2 - x3 + x4 - x5;
    u16::try_from(value.clamp(0, 1000)).unwrap_or(0)
}

pub(crate) fn compute(input: &RelevanceInput<'_>, _policy: &GatePolicy) -> RelevanceScore {
    let semantic = clamp_milli(input.semantic_milli);
    let entity = clamp_milli(input.entity_milli);
    let gain = clamp_milli(input.gain_milli);
    let trust = clamp_milli(input.source_trust_milli);

    let weighted = WEIGHT_SEMANTIC * u32::from(semantic)
        + WEIGHT_ENTITY * u32::from(entity)
        + WEIGHT_GAIN * u32::from(gain)
        + WEIGHT_TRUST * u32::from(trust);
    let base = (weighted + 500) / 1000;
    let decay = u32::from(decay_milli(input.delta_ms));
    let score = (base * decay + 500) / 1000;

    RelevanceScore {
        semantic_milli: semantic,
        entity_milli: entity,
        gain_milli: gain,
        source_trust_milli: trust,
        decay_milli: u16::try_from(decay).unwrap_or(1000),
        score_milli: u16::try_from(score.min(1000)).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_series_hits_boundaries() {
        assert_eq!(decay_milli(0), 1000);
        assert!((367..=368).contains(&decay_milli(180_000)));
    }
}
