//! 关键词候选派生与出卡准入：只读、无 IO、无随机。

use std::collections::BTreeSet;

use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::gate::{AuthorizationStatus, GateCounters, GateDecision};
use meeting_copilot_domain::model::gate_keyword::{
    keyword_kind_of, AllowTraceRef, CitationAlignment, ClickDisabledReason, ClickState,
    KeywordCandidate, KeywordCardStatus,
};
use meeting_copilot_domain::vo::{CardRef, KeywordRef, NormalizedTerm, SessionId, TitleText};

use crate::gate::input::GapScanInput;
use crate::gate::prefilter;

pub struct KeywordReobservation {
    pub keyword_ref: KeywordRef,
    pub dedupe_key: DedupeKey,
    pub session_id: SessionId,
}

pub struct KeywordCandidateSet {
    pub new_candidates: Vec<KeywordCandidate>,
    pub reobservations: Vec<KeywordReobservation>,
}

/// 五类候选派生：未答提问直接丢弃；同会话同术语首见唯一。
pub fn derive_keyword_candidates(scan: &GapScanInput<'_>) -> KeywordCandidateSet {
    let mut counters = GateCounters::default();
    let scanned = prefilter::scan_candidates_with_rule(scan, &mut counters);
    let mut seen: BTreeSet<(SessionId, NormalizedTerm)> = BTreeSet::new();
    let mut keywords = BTreeMapLite::new();
    let mut new_candidates = Vec::new();
    let mut reobservations = Vec::new();

    for item in scanned {
        let Some(kind) = keyword_kind_of(item.rule) else {
            continue;
        };
        let normalized = match normalize_text(&item.term) {
            Some(value) => value,
            None => continue,
        };
        let term = match NormalizedTerm::new(normalized.clone()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let key = (scan.session.clone(), term.clone());
        let dedupe_key = item.candidate.dedupe_key.clone();
        if !seen.insert(key) {
            if let Some((keyword_ref, first_dedupe)) = keywords.get(&(scan.session.clone(), term)) {
                reobservations.push(KeywordReobservation {
                    keyword_ref: keyword_ref.clone(),
                    dedupe_key: first_dedupe.clone(),
                    session_id: scan.session.clone(),
                });
            }
            continue;
        }
        let keyword_text = match TitleText::new(item.term.clone()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let keyword_ref = derive_keyword_ref(scan.session, &dedupe_key);
        keywords.insert(
            (scan.session.clone(), term.clone()),
            (keyword_ref.clone(), dedupe_key.clone()),
        );
        new_candidates.push(KeywordCandidate {
            keyword_ref,
            keyword_text,
            normalized_term: term,
            keyword_kind: kind,
            trigger_span: item.candidate.trigger_span,
            confidence: item.candidate.confidence,
            dedupe_key,
            state: item.candidate.state,
            session_id: scan.session.clone(),
            source_insight_ref: None,
        });
    }

    new_candidates.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.trigger_span.start_ms.cmp(&right.trigger_span.start_ms))
            .then(left.dedupe_key.as_str().cmp(right.dedupe_key.as_str()))
    });
    KeywordCandidateSet {
        new_candidates,
        reobservations,
    }
}

struct BTreeMapLite {
    entries: Vec<((SessionId, NormalizedTerm), (KeywordRef, DedupeKey))>,
}

impl BTreeMapLite {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn insert(&mut self, key: (SessionId, NormalizedTerm), value: (KeywordRef, DedupeKey)) {
        self.entries.push((key, value));
    }

    fn get(&self, key: &(SessionId, NormalizedTerm)) -> Option<&(KeywordRef, DedupeKey)> {
        self.entries
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, value)| value)
    }
}

fn normalize_text(raw: &str) -> Option<String> {
    let lowered = raw.trim().to_lowercase();
    let compact: String = lowered.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

pub fn derive_keyword_ref(session: &SessionId, dedupe_key: &DedupeKey) -> KeywordRef {
    let mut bytes = session.as_str().as_bytes().to_vec();
    bytes.extend_from_slice(dedupe_key.as_str().as_bytes());
    KeywordRef::new(format!("kw-{:016x}", fnv1a64(&bytes)))
        .unwrap_or_else(|_| KeywordRef::new("kw-0000000000000000").expect("fallback"))
}

pub fn derive_card_id(session: &SessionId, dedupe_key: &DedupeKey) -> CardRef {
    let mut forward = session.as_str().as_bytes().to_vec();
    forward.extend_from_slice(dedupe_key.as_str().as_bytes());
    let mut backward = dedupe_key.as_str().as_bytes().to_vec();
    backward.extend_from_slice(session.as_str().as_bytes());
    let hi = fnv1a64(&forward);
    let lo = fnv1a64(&backward);
    CardRef::new(format!("card{hi:016x}{lo:016x}"))
        .unwrap_or_else(|_| CardRef::new("card00000000000000000000000000000000").expect("fallback"))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone, Copy)]
pub struct ClickFacts {
    pub allow_ref_present: bool,
    pub alignment: CitationAlignment,
    pub card_status: KeywordCardStatus,
    pub session_matches: bool,
    pub authorization_granted: bool,
    pub authorization_revoked: bool,
    pub screen_share: bool,
    pub emergency_masked: bool,
    pub reference_stale: bool,
}

/// 六因固定优先级，fail-closed。
pub fn resolve_click_state(facts: ClickFacts) -> ClickState {
    if facts.reference_stale || !facts.session_matches {
        return ClickState::Disabled(ClickDisabledReason::StaleReference);
    }
    if facts.authorization_revoked {
        return ClickState::Disabled(ClickDisabledReason::AuthorizationRevoked);
    }
    if !facts.allow_ref_present
        || !facts.authorization_granted
        || facts.card_status == KeywordCardStatus::Withdrawn
    {
        return ClickState::Disabled(ClickDisabledReason::NotAllowed);
    }
    if facts.emergency_masked {
        return ClickState::Disabled(ClickDisabledReason::EmergencyMasked);
    }
    if facts.screen_share {
        return ClickState::Disabled(ClickDisabledReason::ScreenShareProtection);
    }
    if facts.alignment != CitationAlignment::Passed {
        return ClickState::Disabled(ClickDisabledReason::ReferenceUnverified);
    }
    ClickState::Enabled
}

pub struct KeywordCardEnv<'a> {
    pub authorization_status: AuthorizationStatus,
    pub authorization_revoked: bool,
    pub screen_share: bool,
    pub emergency_masked: bool,
    pub current_session: &'a SessionId,
    pub card_status: KeywordCardStatus,
}

pub struct KeywordCardAdmission {
    pub card_drafts: bool,
    pub click_state: ClickState,
    pub allow_ref: Option<AllowTraceRef>,
    pub body_visible: bool,
}

pub fn keyword_card_admission(
    candidate: &KeywordCandidate,
    decision: &GateDecision,
    trace_id: &meeting_copilot_domain::vo::TraceId,
    alignment: CitationAlignment,
    env: &KeywordCardEnv<'_>,
) -> KeywordCardAdmission {
    if matches!(decision, GateDecision::Deny { .. }) {
        return KeywordCardAdmission {
            card_drafts: false,
            click_state: ClickState::Disabled(ClickDisabledReason::NotAllowed),
            allow_ref: None,
            body_visible: false,
        };
    }
    let allow_ref = match decision {
        GateDecision::Allow { stopped_at, .. } => {
            Some(AllowTraceRef::with_trace(trace_id.clone(), *stopped_at))
        }
        GateDecision::Deny { .. } => None,
    };
    let body_visible = !env.screen_share && !env.emergency_masked;
    let session_matches = &candidate.session_id == env.current_session;
    let facts = ClickFacts {
        allow_ref_present: allow_ref.is_some(),
        alignment,
        card_status: env.card_status,
        session_matches,
        authorization_granted: env.authorization_status == AuthorizationStatus::Granted,
        authorization_revoked: env.authorization_revoked,
        screen_share: env.screen_share,
        emergency_masked: env.emergency_masked,
        reference_stale: !session_matches,
    };

    KeywordCardAdmission {
        card_drafts: true,
        click_state: resolve_click_state(facts),
        allow_ref,
        body_visible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_is_local_only_shape() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    }
}
