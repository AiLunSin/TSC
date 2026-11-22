//! G1 预筛：11 项会话级硬闸门 + L0 六类规则（确定性、无 IO）。

use std::cmp::Ordering;
use std::collections::BTreeSet;

use meeting_copilot_domain::model::card::{DedupeKey, GapCandidate};
use meeting_copilot_domain::model::enums::{
    AutoSuggestionSwitch, ConsentMode, GapKind, GapState, SessionState,
};
use meeting_copilot_domain::model::gate::{
    GapRule, GateCounter, GateCounters, GatePolicy, GateRejectReason, GateStage,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::Confidence;

use crate::gate::input::{GapScanInput, GateInput};

pub(crate) struct PrefilterOutcome {
    pub deny: Option<GateRejectReason>,
    pub candidates: Vec<GapCandidate>,
    pub counters: GateCounters,
}

const INSTITUTION_SUFFIXES: [&str; 9] = [
    "公司",
    "大学",
    "学院",
    "研究院",
    "委员会",
    "部门",
    "集团",
    "局",
    "厅",
];
const POLICY_MARKERS: [&str; 6] = ["GB/T", "GB", "ISO", "RFC", "IEEE", "标准"];
const METRIC_UNITS: [&str; 7] = ["%", "ms", "秒", "分钟", "元", "个", "次"];
const QUESTION_MARKERS: [&str; 5] = ["怎么", "如何", "为什么", "怎么办", "吗"];

/// 先硬闸门、后六规则；一个输入只产一个原因码。
pub(crate) fn run(input: &GateInput<'_>, _policy: &GatePolicy) -> PrefilterOutcome {
    let mut counters = GateCounters::default();
    if let Some(reason) = hard_gate(input) {
        if let Some(counter) = reason.counter_at(GateStage::G1) {
            counters.bump(counter);
        }
        return PrefilterOutcome {
            deny: Some(reason),
            candidates: Vec::new(),
            counters,
        };
    }

    let candidates = scan_candidates(&input.scan, &mut counters);
    PrefilterOutcome {
        deny: None,
        candidates,
        counters,
    }
}

fn hard_gate(input: &GateInput<'_>) -> Option<GateRejectReason> {
    if !input.auto_suggestion_enabled
        || input.policy.auto_suggestion != AutoSuggestionSwitch::Enabled
    {
        return Some(GateRejectReason::AutoDisabled);
    }
    if input.policy.consent == ConsentMode::Explicit && !input.consent_confirmed {
        return Some(GateRejectReason::ConsentMissing);
    }
    if input.session_state == SessionState::Paused {
        return Some(GateRejectReason::Paused);
    }
    if input.session_state == SessionState::Silent || input.silent_period {
        return Some(GateRejectReason::SilentPeriod);
    }
    if input.screen_share && input.policy.screen_share_protection_default {
        return Some(GateRejectReason::ScreenShareProtection);
    }
    if input
        .fragments
        .iter()
        .any(|fragment| fragment.confidence.value() < 0.5)
    {
        return Some(GateRejectReason::PreFilterLowConfidence);
    }
    if input.session_suppressed {
        return Some(GateRejectReason::Suppressed);
    }
    if input.budget.global_window_used >= 5 {
        return Some(GateRejectReason::QuotaExhausted);
    }
    if input.duplicate_detected {
        return Some(GateRejectReason::Duplicate);
    }
    if input.provider.enabled_count == 0 {
        return Some(GateRejectReason::NoProviderEnabled);
    }
    if sensitive_term_present(&input.scan) {
        return Some(GateRejectReason::SensitiveTerm);
    }
    None
}

fn sensitive_term_present(scan: &GapScanInput<'_>) -> bool {
    scan.fragments.iter().any(|fragment| {
        let text = fragment.text.as_str();
        scan.sensitive_terms
            .iter()
            .any(|term| !term.is_empty() && text.contains(term.as_str()))
    })
}

/// 诊断入口：产出候选并回写 32 项计数。
pub fn detect_gaps_with_counters(
    scan: &GapScanInput<'_>,
    counters: &mut GateCounters,
) -> Vec<GapCandidate> {
    scan_candidates(scan, counters)
}

fn scan_candidates(scan: &GapScanInput<'_>, counters: &mut GateCounters) -> Vec<GapCandidate> {
    let mut out = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    for fragment in scan.fragments {
        let reference = fragment.reference.clone();
        let text = fragment.text.as_str();

        if fragment.confidence.value() < 0.5 {
            counters.bump(GateCounter::GateL0LowConfidence);
            continue;
        }

        let stable_ms = scan
            .stable_prefix_ms
            .get(&reference)
            .copied()
            .unwrap_or(fragment.span.end_ms);
        if stable_ms.saturating_add(2_000) > scan.now_ms {
            continue;
        }

        for (rule, term) in detect_terms(scan, text) {
            if !seen.insert(term.clone()) {
                continue;
            }
            if scan.explained_terms.iter().any(|known| known == &term) {
                continue;
            }
            if is_sensitive(scan, &term) {
                counters.bump(GateCounter::GateL0SensitiveSeedBlocked);
                continue;
            }
            let confidence =
                Confidence::new(f64::from(rule.base_milli()) / 1000.0).unwrap_or(Confidence::ZERO);
            if confidence.value() < 0.5 {
                counters.bump(GateCounter::GateL0LowConfidence);
                continue;
            }
            out.push(GapCandidate {
                kind: gap_kind(rule),
                trigger_span: fragment.span,
                confidence,
                dedupe_key: derive_dedupe_key(rule, &term, fragment.span),
                state: GapState::Detected,
            });
        }

        if looks_like_question(text) {
            if scan.answered_at_ms.contains_key(&reference) {
                counters.bump(GateCounter::GateL0QuestionAnsweredSkipped);
            } else if scan.now_ms.saturating_sub(fragment.span.end_ms)
                >= GapScanInput::QUESTION_WAIT_MS
            {
                let confidence = Confidence::new(0.85).unwrap_or(Confidence::ZERO);
                out.push(GapCandidate {
                    kind: GapKind::UnansweredQuestion,
                    trigger_span: fragment.span,
                    confidence,
                    dedupe_key: derive_dedupe_key(
                        GapRule::UnansweredQuestion,
                        "unanswered_question",
                        fragment.span,
                    ),
                    state: GapState::Detected,
                });
            }
        }
    }

    out.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(Ordering::Equal)
            .then(left.trigger_span.start_ms.cmp(&right.trigger_span.start_ms))
            .then(left.dedupe_key.as_str().cmp(right.dedupe_key.as_str()))
    });

    for _ in 0..out.len() {
        counters.bump(GateCounter::GateL0Candidates);
    }
    out
}

fn detect_terms(scan: &GapScanInput<'_>, text: &str) -> Vec<(GapRule, String)> {
    let mut terms = Vec::new();

    for name in scan.lexicon_names {
        if !name.is_empty() && text.contains(name.as_str()) {
            terms.push((GapRule::ProperNoun, name.clone()));
        }
    }

    let acronym = parenthesized_acronym(text);
    if let Some(term) = &acronym {
        terms.push((GapRule::Acronym, term.clone()));
    }

    for token in tokens(text) {
        if token.chars().count() < 2 {
            continue;
        }
        if is_abbreviation(&token)
            && !whitelisted(scan, &token)
            && acronym.as_deref() != Some(token.as_str())
        {
            terms.push((GapRule::Abbreviation, token.clone()));
        } else if let Some(term) = policy_standard_term(&token) {
            terms.push((GapRule::PolicyStandard, term));
        }
        if let Some(term) = metric_term(&token) {
            terms.push((GapRule::Metric, term));
        }
    }

    if let Some(term) = institution_term(text) {
        terms.push((GapRule::Institution, term));
    }

    terms
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '/' || c == '%'))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn is_abbreviation(token: &str) -> bool {
    let count = token.chars().count();
    (2..=6).contains(&count) && token.chars().all(|c| c.is_ascii_uppercase())
}

/// 括号展开模式：`ABBR（全称）` 或 `全称 (ABBR)`。
fn parenthesized_acronym(text: &str) -> Option<String> {
    for (index, character) in text.char_indices() {
        if character == '(' || character == '（' {
            let prefix = &text[..index];
            let mut token: Option<&str> = None;
            for item in prefix.split(|c: char| !c.is_alphanumeric()) {
                if !item.is_empty() {
                    token = Some(item);
                }
            }
            if let Some(candidate) = token {
                if is_abbreviation(candidate) {
                    return Some(candidate.to_owned());
                }
            }
        }
    }
    None
}

fn whitelisted(scan: &GapScanInput<'_>, token: &str) -> bool {
    scan.acronym_whitelist.iter().any(|item| item == token)
}

fn policy_standard_term(token: &str) -> Option<String> {
    POLICY_MARKERS
        .iter()
        .find(|marker| token == **marker || token.starts_with(*marker))
        .map(|marker| (*marker).to_owned())
}

fn metric_term(token: &str) -> Option<String> {
    if !token.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    if METRIC_UNITS.iter().any(|unit| token.contains(unit)) {
        return Some(token.to_owned());
    }
    None
}

fn institution_term(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    for suffix in INSTITUTION_SUFFIXES {
        let suffix_chars: Vec<char> = suffix.chars().collect();
        if chars.len() < suffix_chars.len() {
            continue;
        }
        if chars.ends_with(&suffix_chars) {
            let start = chars.len().saturating_sub(suffix_chars.len() + 3);
            return Some(chars[start..].iter().collect());
        }
        if text.contains(suffix) {
            let index = text.find(suffix)?;
            let prefix = &text[..index];
            let prefix_chars: String = prefix
                .chars()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            return Some(format!("{prefix_chars}{suffix}"));
        }
    }
    None
}

fn looks_like_question(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.ends_with('?')
        || trimmed.ends_with('？')
        || QUESTION_MARKERS
            .iter()
            .any(|marker| trimmed.contains(marker))
}

fn is_sensitive(scan: &GapScanInput<'_>, term: &str) -> bool {
    scan.sensitive_terms
        .iter()
        .any(|sensitive| sensitive == term)
}

fn gap_kind(rule: GapRule) -> GapKind {
    match rule {
        GapRule::ProperNoun | GapRule::Institution => GapKind::ProperNoun,
        GapRule::Abbreviation => GapKind::Abbreviation,
        GapRule::PolicyStandard => GapKind::PolicyStandard,
        GapRule::Metric => GapKind::Metric,
        GapRule::UnansweredQuestion => GapKind::UnansweredQuestion,
        GapRule::Acronym => GapKind::Acronym,
    }
}

fn derive_dedupe_key(rule: GapRule, term: &str, span: TextRange) -> DedupeKey {
    let rule_tag = match rule {
        GapRule::ProperNoun => "proper_noun",
        GapRule::Abbreviation => "abbreviation",
        GapRule::Institution => "institution",
        GapRule::PolicyStandard => "policy_standard",
        GapRule::Metric => "metric",
        GapRule::UnansweredQuestion => "unanswered_question",
        GapRule::Acronym => "acronym",
    };
    let hi = fnv1a(
        0xcbf2_9ce4_8422_2325,
        rule_tag
            .as_bytes()
            .iter()
            .copied()
            .chain(term.as_bytes().iter().copied()),
    );
    let lo = fnv1a(
        0x8422_2325_cbf2_9ce4,
        span.start_ms
            .to_le_bytes()
            .iter()
            .copied()
            .chain(term.as_bytes().iter().copied()),
    );
    DedupeKey::new(format!("{hi:016x}{lo:016x}"))
        .unwrap_or_else(|_| DedupeKey::new("0".repeat(32)).expect("32 hex fallback"))
}

fn fnv1a(seed: u64, bytes: impl Iterator<Item = u8>) -> u64 {
    let mut hash = seed;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_keys_are_32_hex_and_deterministic() {
        let span = TextRange {
            start_ms: 10,
            end_ms: 20,
        };
        let left = derive_dedupe_key(GapRule::Metric, "30%", span);
        let right = derive_dedupe_key(GapRule::Metric, "30%", span);
        assert_eq!(left, right);
        assert_eq!(left.as_str().len(), 32);
        assert!(left.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }
}
