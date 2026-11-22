//! G4 预算与冷却：只读快照，推进在判定外（`GateBudget::commit`）。

use meeting_copilot_domain::model::gate::{
    EntityKey, FrequencyProfile, GatePolicy, GateRejectReason, NetworkState,
};

use crate::gate::input::GateInput;

pub(crate) struct BudgetOutcome {
    pub deny: Option<(GateRejectReason, Option<u64>)>,
    pub queued: bool,
    pub remaining: u16,
}

const TOKEN_CAPACITY_MILLI: u16 = 3000;
const WINDOW_MS: u64 = 600_000;
const TOPIC_QUOTA: u8 = 2;
const INFLIGHT_LIMIT: u8 = 3;

/// 整数令牌桶；容量上限 3000 毫令牌。
pub fn refill(
    tokens_milli: u16,
    window_start_ms: u64,
    now_ms: u64,
    frequency: FrequencyProfile,
) -> u16 {
    let elapsed = now_ms.saturating_sub(window_start_ms);
    let add = u128::from(elapsed)
        .saturating_mul(u128::from(frequency.per_window()))
        .saturating_mul(1000)
        / u128::from(WINDOW_MS);
    let add = u16::try_from(add).unwrap_or(TOKEN_CAPACITY_MILLI);
    tokens_milli.saturating_add(add).min(TOKEN_CAPACITY_MILLI)
}

pub(crate) fn check(input: &GateInput<'_>, policy: &GatePolicy) -> BudgetOutcome {
    let remaining = input
        .budget
        .session_remaining_with(policy.session_outbound_cap);

    if input.network == NetworkState::Offline {
        return deny(GateRejectReason::Offline, None, remaining);
    }
    if input.provider.enabled_count == 0 {
        return deny(GateRejectReason::NoProviderEnabled, None, remaining);
    }
    if input.llm.cooling {
        return deny(
            GateRejectReason::GatewayCooling,
            input.llm.cooling_until_ms,
            remaining,
        );
    }
    if input.budget.session_outbound_used >= policy.session_outbound_cap {
        return deny(GateRejectReason::BudgetExhausted, None, remaining);
    }

    let window_elapsed = input.now_ms.saturating_sub(input.budget.window_start_ms);
    let window_reset = window_elapsed >= WINDOW_MS;
    let tokens = if window_reset {
        TOKEN_CAPACITY_MILLI
    } else {
        refill(
            input.budget.tokens_milli,
            input.budget.window_start_ms,
            input.now_ms,
            policy.frequency_profile,
        )
    };
    if tokens < 1000 {
        return deny(GateRejectReason::BudgetExhausted, None, remaining);
    }

    let global_used = if window_reset {
        0
    } else {
        input.budget.global_window_used
    };
    if global_used >= policy.frequency_profile.per_window() {
        return deny(GateRejectReason::BudgetExhausted, None, remaining);
    }

    let topic_used = if window_reset {
        0
    } else {
        input
            .budget
            .topic_window_used
            .get(&input.signals.cluster_id)
            .copied()
            .unwrap_or(0)
    };
    if topic_used >= TOPIC_QUOTA {
        return deny(GateRejectReason::BudgetExhausted, None, remaining);
    }

    let mut cooldown_until = None;
    for key in input.signals.entity_keys {
        if let Some(entity) = EntityKey::parse(key) {
            if let Some(last) = input.budget.entity_last_seen_ms.get(&entity) {
                if input.now_ms.saturating_sub(*last) < 300_000 {
                    cooldown_until = Some(last.saturating_add(300_000));
                }
            }
        }
    }
    if cooldown_until.is_some() {
        return deny(GateRejectReason::BudgetExhausted, cooldown_until, remaining);
    }

    BudgetOutcome {
        deny: None,
        queued: input.budget.inflight >= INFLIGHT_LIMIT,
        remaining,
    }
}

fn deny(reason: GateRejectReason, cooldown_until_ms: Option<u64>, remaining: u16) -> BudgetOutcome {
    BudgetOutcome {
        deny: Some((reason, cooldown_until_ms)),
        queued: false,
        remaining,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refill_caps_at_3000() {
        assert_eq!(
            refill(0, 0, 600_001, FrequencyProfile::Medium),
            TOKEN_CAPACITY_MILLI
        );
        assert_eq!(refill(0, 0, 0, FrequencyProfile::Low), 0);
    }
}
