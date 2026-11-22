//! 回放夹具：12 项纯数据结构，无 runner、无 IO、无随机源。

use serde::{Deserialize, Serialize};

use crate::model::enums::{InsightKind, SessionState, TriggerKind};
use crate::model::gate::{
    AuthorizationStatus, GapRule, GateContentType, GatePolicy, GateRejectReason, GateStage,
    NetworkState,
};
use crate::model::session::TextRange;

/// 回放时钟：第 n 次取值 = anchor_ms + n × tick_ms。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockFixture {
    pub anchor_ms: u64,
    pub tick_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentFixture {
    pub span: TextRange,
    pub confidence_milli: u16,
    pub dual_channel: bool,
    #[serde(default)]
    pub answered_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsightFixture {
    pub reference_digest: String,
    pub kind: InsightKind,
    pub confidence_milli: u16,
    pub manual_locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetFixture {
    pub session_outbound_used: u16,
    pub global_window_used: u8,
    #[serde(default)]
    pub topic_window_used: Vec<(u32, u8)>,
    #[serde(default)]
    pub entity_last_seen_ms: Vec<(String, u64)>,
    pub inflight: u8,
    #[serde(default)]
    pub tokens_milli: u16,
    #[serde(default)]
    pub window_start_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationFixture {
    pub status: AuthorizationStatus,
    #[serde(default)]
    pub granted_at_ms: Option<i64>,
    #[serde(default)]
    pub content_types: Vec<GateContentType>,
    #[serde(default)]
    pub max_query_chars: Option<u16>,
    #[serde(default)]
    pub provider_ids: Vec<String>,
    #[serde(default)]
    pub notice_version: u16,
    #[serde(default)]
    pub revoked_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelevanceFixture {
    pub semantic_milli: u16,
    pub entity_milli: u16,
    pub gain_milli: u16,
    pub source_trust_milli: u16,
    pub delta_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SuppressionFixture {
    #[serde(default)]
    pub topics: Vec<u32>,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSeedFixture {
    pub rule: GapRule,
    pub confidence_milli: u16,
    pub dedupe_key: String,
    pub topic_cluster_id: u32,
    #[serde(default)]
    pub entity_keys: Vec<String>,
    #[serde(default)]
    pub primary_domain: Option<String>,
}

/// 全输入闭包：一次判定所需的全部外部状态（23 字段）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateInputFixture {
    pub now_ms: u64,
    pub clock: ClockFixture,
    pub trigger: TriggerKind,
    pub session_state: SessionState,
    pub silent_period: bool,
    pub screen_share: bool,
    pub consent_confirmed: bool,
    pub auto_suggestion_enabled: bool,
    pub fragments: Vec<FragmentFixture>,
    pub insights: Vec<InsightFixture>,
    pub budget: BudgetFixture,
    pub authorization: AuthorizationFixture,
    pub provider_enabled_count: u8,
    #[serde(default)]
    pub provider_ids: Vec<String>,
    pub llm_cooling: bool,
    #[serde(default)]
    pub llm_cooling_until_ms: Option<u64>,
    pub network: NetworkState,
    pub audit_available: bool,
    pub relevance: RelevanceFixture,
    pub suppression: SuppressionFixture,
    pub candidate_seeds: Vec<CandidateSeedFixture>,
    #[serde(default)]
    pub cache_age_ms: Option<u64>,
    #[serde(default)]
    pub refresh_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedDecision {
    Allow,
    Deny {
        reason: GateRejectReason,
        stopped_at: GateStage,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReplayCase {
    pub case_id: String,
    pub input_fixture: GateInputFixture,
    pub policy_fixture: GatePolicy,
    pub expected: ExpectedDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayReport {
    pub total: u32,
    pub matched: u32,
    pub mismatched: u32,
    pub mismatch_cases: Vec<String>,
    pub seed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::enums::SessionState;
    use crate::model::gate::AuthorizationStatus;

    fn empty_input() -> GateInputFixture {
        GateInputFixture {
            now_ms: 0,
            clock: ClockFixture {
                anchor_ms: 0,
                tick_ms: 1_000,
            },
            trigger: TriggerKind::Auto,
            session_state: SessionState::Listening,
            silent_period: false,
            screen_share: false,
            consent_confirmed: true,
            auto_suggestion_enabled: true,
            fragments: Vec::new(),
            insights: Vec::new(),
            budget: BudgetFixture {
                session_outbound_used: 0,
                global_window_used: 0,
                topic_window_used: Vec::new(),
                entity_last_seen_ms: Vec::new(),
                inflight: 0,
                tokens_milli: 0,
                window_start_ms: 0,
            },
            authorization: AuthorizationFixture {
                status: AuthorizationStatus::Granted,
                granted_at_ms: Some(0),
                content_types: vec![GateContentType::TextQuery],
                max_query_chars: Some(200),
                provider_ids: vec!["provider-1".to_owned()],
                notice_version: 1,
                revoked_at_ms: None,
            },
            provider_enabled_count: 1,
            provider_ids: vec!["provider-1".to_owned()],
            llm_cooling: false,
            llm_cooling_until_ms: None,
            network: NetworkState::Online,
            audit_available: true,
            relevance: RelevanceFixture {
                semantic_milli: 0,
                entity_milli: 0,
                gain_milli: 0,
                source_trust_milli: 0,
                delta_ms: 0,
            },
            suppression: SuppressionFixture::default(),
            candidate_seeds: Vec::new(),
            cache_age_ms: None,
            refresh_available: false,
        }
    }

    #[test]
    fn gate_input_fixture_covers_all_input_state() {
        let digest = "a".repeat(32);
        let cases = [
            (SessionState::Listening, NetworkState::Online, false),
            (SessionState::Paused, NetworkState::Offline, true),
            (SessionState::Silent, NetworkState::Unknown, false),
            (SessionState::NotStarted, NetworkState::Online, true),
            (SessionState::Ended, NetworkState::Offline, false),
            (SessionState::Deleted, NetworkState::Unknown, true),
            (SessionState::Listening, NetworkState::Offline, true),
            (SessionState::Paused, NetworkState::Unknown, false),
        ];
        for (session_state, network, cooling) in cases {
            let mut input = empty_input();
            input.session_state = session_state;
            input.network = network;
            input.llm_cooling = cooling;
            input.fragments.push(FragmentFixture {
                span: TextRange {
                    start_ms: 0,
                    end_ms: 1_000,
                },
                confidence_milli: 900,
                dual_channel: true,
                answered_ms: Some(500),
            });
            input.insights.push(InsightFixture {
                reference_digest: digest.clone(),
                kind: InsightKind::KeyInfo,
                confidence_milli: 800,
                manual_locked: false,
            });
            input.candidate_seeds.push(CandidateSeedFixture {
                rule: GapRule::ProperNoun,
                confidence_milli: 700,
                dedupe_key: digest.clone(),
                topic_cluster_id: 1,
                entity_keys: vec![digest.clone()],
                primary_domain: Some("example.com".to_owned()),
            });
            input.suppression = SuppressionFixture {
                topics: vec![1],
                providers: vec!["provider-1".to_owned()],
                entities: vec![digest.clone()],
            };
            let first = serde_json::to_string(&input).expect("first");
            let decoded: GateInputFixture = serde_json::from_str(&first).expect("decode");
            let second = serde_json::to_string(&decoded).expect("second");
            assert_eq!(first, second);
        }
    }

    #[test]
    fn replay_case_keeps_closed_expected_decision() {
        let case = GateReplayCase {
            case_id: "case-1".to_owned(),
            input_fixture: empty_input(),
            policy_fixture: GatePolicy::default(),
            expected: ExpectedDecision::Deny {
                reason: GateRejectReason::PolicyUnavailable,
                stopped_at: GateStage::G1,
            },
        };
        let encoded = serde_json::to_string(&case).expect("encode");
        let decoded: GateReplayCase = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, case);
    }
}
