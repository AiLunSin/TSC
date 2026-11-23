//! 卡片生成：把一段会议文本提取为关键词，并快速产出可点击卡片。
//!
//! 来源为 `UserGenerated`（本地会议片段），因此卡片可合法进入可点击态；
//! 不伪造外部检索来源。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use jieba_rs::{Jieba, KeywordExtract, KeywordExtractConfig, TfIdf};
use meeting_copilot_application::gate::keyword::{keyword_card_admission, KeywordCardEnv};
use meeting_copilot_application::settings::{CardSnapshotEntry, CardSnapshotSource};
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::{GapState, TriggerKind};
use meeting_copilot_domain::model::gate::{AuthorizationStatus, GateDecision};
use meeting_copilot_domain::model::gate_keyword::{
    CitationAlignment, KeywordCandidate, KeywordCardDraft, KeywordCardStatus, KeywordKind,
    ReferenceAccessStatus, ReferenceTrustTier, ReferenceView,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::{
    CardRef, ConclusionText, Confidence, KeywordRef, NormalizedTerm, ReferenceRef, SessionId,
    TitleText,
};

use crate::ipc::card_gate::{self, LocalGate};
use crate::ipc::dto::IpcErrorDto;

const STOPWORDS: &[&str] = &[
    "我们", "你们", "他们", "这个", "那个", "就是", "然后", "可以", "一个", "什么", "怎么", "现在",
    "今天", "大家", "比较", "觉得", "其实", "因为", "所以", "如果", "但是", "而且", "还有", "没有",
    "不是", "应该", "需要", "进行", "通过", "之后", "之前", "时候", "问题", "东西", "事情", "这样",
    "那样", "这里", "那里", "一下", "目前", "已经", "还是", "或者", "的话", "对吧", "是啊", "好的",
    "明白", "收到", "谢谢",
];

const MAX_KEYWORDS_PER_TEXT: usize = 8;
const MAX_CARDS: usize = 200;

/// 可变卡片源：内存内追加，供 `keyword_cards_snapshot` 只读投影。
///
/// 由 Tauri managed state 持有（`Arc<CardStore>`），不再使用进程级静态单例。
pub struct CardStore {
    cards: Mutex<Vec<CardSnapshotEntry>>,
    sequence: AtomicU64,
    jieba: Jieba,
    tfidf: TfIdf,
}

impl CardStore {
    pub(crate) fn new() -> Self {
        Self {
            cards: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
            jieba: Jieba::new(),
            tfidf: TfIdf::new(None::<&mut &[u8]>, KeywordExtractConfig::default()),
        }
    }

    pub fn ingest(&self, text: &str) -> Result<usize, IpcErrorDto> {
        let keywords = self.extract_keywords(text);
        if keywords.is_empty() {
            return Err(IpcErrorDto::new(
                "no_keyword",
                "未从文本中识别到关键词。",
                false,
            ));
        }
        let excerpt: String = text.trim().chars().take(80).collect();
        let now = now_ms();
        let Some(gate) = card_gate::evaluate(text, self.sequence.load(Ordering::Relaxed), now)
        else {
            return Ok(0);
        };
        if matches!(&gate.decision, GateDecision::Deny { .. }) {
            return Ok(0);
        }
        let mut store = self
            .cards
            .lock()
            .map_err(|_| IpcErrorDto::new("card_store_unavailable", "卡片存储不可用。", true))?;
        let mut added = 0usize;
        for keyword in keywords.into_iter().take(MAX_KEYWORDS_PER_TEXT) {
            if store.len() >= MAX_CARDS {
                store.remove(0);
            }
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
            match build_draft(sequence, &keyword, &excerpt, &gate) {
                Some(draft) => {
                    store.push(CardSnapshotEntry::Draft(draft));
                    added += 1;
                }
                None => continue,
            }
        }
        Ok(added)
    }

    /// 从会议文本中提取关键词：jieba TF-IDF 排序，过滤停用词与单字。
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
        for keyword in
            self.tfidf
                .extract_keywords(&self.jieba, text, MAX_KEYWORDS_PER_TEXT, Vec::new())
        {
            let term = keyword.keyword.trim().to_owned();
            if term.chars().count() < 2 || STOPWORDS.contains(&term.as_str()) {
                continue;
            }
            if result
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&term))
            {
                continue;
            }
            result.push(term);
            if result.len() >= MAX_KEYWORDS_PER_TEXT {
                break;
            }
        }
        result
    }
}

impl CardSnapshotSource for CardStore {
    fn cards(&self) -> Vec<CardSnapshotEntry> {
        self.cards
            .lock()
            .map(|cards| cards.clone())
            .unwrap_or_default()
    }
}

fn build_draft(
    sequence: u64,
    keyword: &str,
    excerpt: &str,
    gate: &LocalGate,
) -> Option<KeywordCardDraft> {
    let title: String = keyword.chars().take(TitleText::LIMIT).collect();
    let summary: String = excerpt.chars().take(ConclusionText::LIMIT).collect();
    let keyword_text = TitleText::new(title).ok()?;
    let normalized_term = NormalizedTerm::new(keyword).ok()?;
    let candidate = KeywordCandidate {
        keyword_ref: KeywordRef::new(format!("kw-{sequence:016}")).ok()?,
        keyword_text: keyword_text.clone(),
        normalized_term: normalized_term.clone(),
        keyword_kind: classify_kind(keyword),
        trigger_span: TextRange {
            start_ms: 0,
            end_ms: 1,
        },
        confidence: Confidence::new(0.9).ok()?,
        dedupe_key: DedupeKey::new(format!("{sequence:032x}")).ok()?,
        state: GapState::Detected,
        session_id: gate.session_id.clone(),
        source_insight_ref: None,
    };
    let env = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        current_session: &gate.session_id,
        card_status: KeywordCardStatus::Displayed,
    };
    let admission = keyword_card_admission(
        &candidate,
        &gate.decision,
        &gate.trace_id,
        CitationAlignment::Passed,
        &env,
    );
    if !admission.card_drafts {
        return None;
    }
    Some(KeywordCardDraft {
        card_id: CardRef::new(format!("card-{sequence}")).ok()?,
        keyword_text,
        normalized_term,
        keyword_kind: classify_kind(keyword),
        trigger: TriggerKind::OnDemand,
        triggered_at_ms: now_ms(),
        confidence: Confidence::new(0.9).ok()?,
        summary: ConclusionText::new(summary).ok(),
        references: vec![ReferenceView {
            reference_id: ReferenceRef::new(format!("local-{sequence}")).ok()?,
            trust_tier: ReferenceTrustTier::UserGenerated,
            access_status: ReferenceAccessStatus::Available,
            published_at_ms: None,
            alignment: CitationAlignment::Passed,
        }],
        citation_alignment: CitationAlignment::Passed,
        gate_decision_ref: admission.allow_ref?,
        card_status: KeywordCardStatus::Displayed,
        click_state: admission.click_state,
    })
}

fn classify_kind(keyword: &str) -> KeywordKind {
    if keyword.chars().any(|c| c == '%') {
        return KeywordKind::Metric;
    }
    let letters: String = keyword
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    if !letters.is_empty() && letters == letters.to_uppercase() {
        return KeywordKind::Acronym;
    }
    KeywordKind::ProperNoun
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn ingest_text(
    state: tauri::State<'_, Arc<CardStore>>,
    text: String,
) -> Result<usize, IpcErrorDto> {
    if text.trim().is_empty() {
        return Err(IpcErrorDto::new("empty_text", "文本为空。", false));
    }
    state.ingest(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_chinese_and_acronym_keywords() {
        let store = CardStore::new();
        let keywords =
            store.extract_keywords("我们确认 SLA 99.9% 和灰度范围，阿里云华东两站点优先。");
        assert!(!keywords.is_empty());
        assert!(keywords
            .iter()
            .any(|item| item.contains("阿里云") || item.contains("灰度") || item == "SLA"));
        assert!(!keywords.iter().any(|item| item == "我们"));
    }

    #[test]
    fn stops_on_stopwords_and_short_tokens() {
        let store = CardStore::new();
        assert!(!store
            .extract_keywords("我们 可以 这个 那个")
            .iter()
            .any(|item| item == "我们"));
        assert!(store.extract_keywords("").is_empty());
    }

    #[test]
    fn gate_allows_local_clickable_card_with_real_allow_trace() {
        let gate = card_gate::evaluate("会议讨论阿里云方案", 1, 10_000).expect("gate");
        assert!(matches!(&gate.decision, GateDecision::Allow { .. }));
        let draft = build_draft(1, "阿里云", "会议讨论阿里云方案", &gate).expect("draft");
        assert_eq!(
            draft.click_state,
            meeting_copilot_domain::model::gate_keyword::ClickState::Enabled
        );
        assert_eq!(draft.citation_alignment, CitationAlignment::Passed);
        assert_eq!(draft.gate_decision_ref.trace_id(), &gate.trace_id);
        assert_eq!(draft.references.len(), 1);
        assert_eq!(
            draft.references[0].trust_tier,
            ReferenceTrustTier::UserGenerated
        );
        assert!(draft.click_state_is_consistent());
        assert!(draft.summary_is_consistent());
    }

    #[test]
    fn denied_gate_produces_no_card() {
        let mut gate = card_gate::evaluate("会议讨论阿里云方案", 1, 10_000).expect("gate");
        gate.decision = GateDecision::Deny {
            reason: meeting_copilot_domain::model::gate::GateRejectReason::ConsentMissing,
            stopped_at: meeting_copilot_domain::model::gate::GateStage::G1,
            per_stage: meeting_copilot_domain::model::gate::PerStageResults::empty(),
            cooldown_until_ms: None,
        };
        assert!(build_draft(1, "阿里云", "会议讨论阿里云方案", &gate).is_none());
    }

    #[test]
    fn ingest_appends_cards_but_rejects_empty_text() {
        let store = CardStore::new();
        let added = store.ingest("飞书 钉钉 腾讯会议").expect("added");
        assert!(added >= 1);
        assert!(!store.cards().is_empty());
        assert!(store.ingest("   ").is_err());
    }
}
