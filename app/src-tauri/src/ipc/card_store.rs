//! 卡片生成：把一段会议文本提取为关键词，并快速产出可点击卡片。
//!
//! 来源为 `UserGenerated`（本地会议片段），因此卡片可合法进入可点击态；
//! 不伪造外部检索来源。

use std::collections::{HashMap, VecDeque};
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
    CardRef, ConclusionText, Confidence, KeywordRef, NormalizedTerm, ReferenceRef, TitleText,
};

use crate::ipc::card_gate::{self, LocalGate};
use crate::ipc::dto::IpcErrorDto;
use crate::privacy::PrivacyMode;

const STOPWORDS: &[&str] = &[
    "我们", "你们", "他们", "这个", "那个", "就是", "然后", "可以", "一个", "什么", "怎么", "现在",
    "今天", "大家", "比较", "觉得", "其实", "因为", "所以", "如果", "但是", "而且", "还有", "没有",
    "不是", "应该", "需要", "进行", "通过", "之后", "之前", "时候", "问题", "东西", "事情", "这样",
    "那样", "这里", "那里", "一下", "目前", "已经", "还是", "或者", "的话", "对吧", "是啊", "好的",
    "明白", "收到", "谢谢",
];

const MAX_KEYWORDS_PER_TEXT: usize = 8;
const MAX_CARDS: usize = 200;

/// 自动出卡限流：同一个词 90 秒内不重复出卡，整场每分钟最多 8 张。
///
/// 只作用于自动路径（ASR 稳定句 / UIA 字幕）；用户手点"生成卡片"不受限。
const AUTO_COOLDOWN_MS: u64 = 90_000;
const AUTO_WINDOW_MS: u64 = 60_000;
const AUTO_CARDS_PER_MINUTE: usize = 8;
/// 冷却表上限：超过后清一遍过期项，避免长会无限增长。
const AUTO_TERM_MEMORY: usize = 512;

#[derive(Default)]
struct AutoEmitState {
    last_by_term: HashMap<String, u64>,
    window: VecDeque<u64>,
}

/// 单次自动出卡的额度判定。
enum Budget {
    Allow,
    /// 同词还在冷却期内，跳过这一张。
    Skip,
    /// 本分钟额度已用完，整批都不再出卡。
    Exhausted,
}

/// 可变卡片源：内存内追加，供 `keyword_cards_snapshot` 只读投影。
///
/// 由 Tauri managed state 持有（`Arc<CardStore>`），不再使用进程级静态单例。
pub struct CardStore {
    cards: Mutex<Vec<CardSnapshotEntry>>,
    sequence: AtomicU64,
    jieba: Jieba,
    tfidf: TfIdf,
    privacy: PrivacyMode,
    /// 自动出卡的冷却与分钟额度；手动出卡不查这里。
    auto: Mutex<AutoEmitState>,
}

impl CardStore {
    pub(crate) fn new() -> Self {
        Self::new_with_privacy(PrivacyMode::from_env())
    }

    /// 显式给定隐私态：测试用固定值，生产由 `new()` 从开发期开关读取。
    pub(crate) fn new_with_privacy(privacy: PrivacyMode) -> Self {
        Self {
            cards: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
            jieba: Jieba::new(),
            tfidf: TfIdf::new(None::<&mut &[u8]>, KeywordExtractConfig::default()),
            privacy,
            auto: Mutex::new(AutoEmitState::default()),
        }
    }

    /// 自动路径（ASR 稳定句、UIA 字幕）：受同词冷却与分钟额度约束。
    pub fn ingest(&self, text: &str) -> Result<usize, IpcErrorDto> {
        self.ingest_inner(text, true)
    }

    /// 手动路径（用户粘贴文本后点"生成卡片"）：不受限流，点了就出。
    pub fn ingest_manual(&self, text: &str) -> Result<usize, IpcErrorDto> {
        self.ingest_inner(text, false)
    }

    fn ingest_inner(&self, text: &str, limited: bool) -> Result<usize, IpcErrorDto> {
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
        let Some(gate) = card_gate::evaluate(
            text,
            self.sequence.load(Ordering::Relaxed),
            now,
            self.privacy,
        ) else {
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
        let mut emitted: Vec<String> = Vec::new();
        for keyword in keywords.into_iter().take(MAX_KEYWORDS_PER_TEXT) {
            if limited {
                match self.check_auto(&keyword, now)? {
                    Budget::Exhausted => break,
                    Budget::Skip => continue,
                    Budget::Allow => {}
                }
            }
            if store.len() >= MAX_CARDS {
                store.remove(0);
            }
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
            match build_draft(sequence, &keyword, &excerpt, &gate, self.privacy) {
                Some(draft) => {
                    store.push(CardSnapshotEntry::Draft(draft));
                    added += 1;
                    if limited {
                        emitted.push(keyword.clone());
                    }
                }
                None => continue,
            }
        }
        drop(store);
        // 只给真正出过卡的词记冷却，避免"被门控拦掉的词也算用掉额度"。
        for term in emitted {
            self.record_auto(&term, now);
        }
        Ok(added)
    }

    /// 自动路径的额度判定：先清过期窗口，再看分钟上限，最后看同词冷却。
    fn check_auto(&self, term: &str, now: u64) -> Result<Budget, IpcErrorDto> {
        let mut auto = self
            .auto
            .lock()
            .map_err(|_| IpcErrorDto::new("card_store_unavailable", "卡片存储不可用。", true))?;
        auto.window
            .retain(|at| now.saturating_sub(*at) < AUTO_WINDOW_MS);
        if auto.window.len() >= AUTO_CARDS_PER_MINUTE {
            return Ok(Budget::Exhausted);
        }
        if let Some(last) = auto.last_by_term.get(term) {
            if now.saturating_sub(*last) < AUTO_COOLDOWN_MS {
                return Ok(Budget::Skip);
            }
        }
        Ok(Budget::Allow)
    }

    fn record_auto(&self, term: &str, now: u64) {
        let Ok(mut auto) = self.auto.lock() else {
            return;
        };
        auto.window.push_back(now);
        if auto.last_by_term.len() >= AUTO_TERM_MEMORY {
            auto.last_by_term
                .retain(|_, at| now.saturating_sub(*at) < AUTO_COOLDOWN_MS);
        }
        auto.last_by_term.insert(term.to_owned(), now);
    }

    /// 用"选中的词 + 模型释义"直接生成一张卡片，供"查意思 → 生成卡片"使用。
    ///
    /// 与 `ingest` 的区别：不抽关键词，标题固定为用户选中的文字，
    /// 结论用释义摘要；门控与隐私判定照旧走 `card_gate::evaluate`。
    pub fn ingest_explanation(&self, word: &str, explanation: &str) -> Result<usize, IpcErrorDto> {
        let word = word.trim();
        let explanation = explanation.trim();
        if word.is_empty() || explanation.is_empty() {
            return Err(IpcErrorDto::new(
                "empty_explanation",
                "释义内容为空。",
                false,
            ));
        }
        let title: String = word.chars().take(TitleText::LIMIT).collect();
        let text = format!("{title} {explanation}");
        let Some(gate) = card_gate::evaluate(
            &text,
            self.sequence.load(Ordering::Relaxed),
            now_ms(),
            self.privacy,
        ) else {
            return Ok(0);
        };
        if matches!(&gate.decision, GateDecision::Deny { .. }) {
            return Ok(0);
        }
        let excerpt: String = explanation.chars().take(80).collect();
        let mut store = self
            .cards
            .lock()
            .map_err(|_| IpcErrorDto::new("card_store_unavailable", "卡片存储不可用。", true))?;
        if store.len() >= MAX_CARDS {
            store.remove(0);
        }
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        match build_draft(sequence, &title, &excerpt, &gate, self.privacy) {
            Some(draft) => {
                store.push(CardSnapshotEntry::Draft(draft));
                Ok(1)
            }
            None => Ok(0),
        }
    }

    /// 清空本次会话的卡片，供界面上的"清空"按钮使用。
    pub fn clear(&self) -> Result<usize, IpcErrorDto> {
        let mut store = self
            .cards
            .lock()
            .map_err(|_| IpcErrorDto::new("card_store_unavailable", "卡片存储不可用。", true))?;
        let removed = store.len();
        store.clear();
        Ok(removed)
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
    privacy: PrivacyMode,
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
        screen_share: privacy.screen_share,
        emergency_masked: privacy.emergency_masked,
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
    state.ingest_manual(&text)
}

/// 清空当前会话的全部卡片，返回被清掉的数量。
#[tauri::command]
pub fn clear_cards(state: tauri::State<'_, Arc<CardStore>>) -> Result<usize, IpcErrorDto> {
    state.clear()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_chinese_and_acronym_keywords() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
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
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        assert!(!store
            .extract_keywords("我们 可以 这个 那个")
            .iter()
            .any(|item| item == "我们"));
        assert!(store.extract_keywords("").is_empty());
    }

    #[test]
    fn gate_allows_local_clickable_card_with_real_allow_trace() {
        let gate = card_gate::evaluate("会议讨论阿里云方案", 1, 10_000, PrivacyMode::default())
            .expect("gate");
        assert!(matches!(&gate.decision, GateDecision::Allow { .. }));
        let draft = build_draft(
            1,
            "阿里云",
            "会议讨论阿里云方案",
            &gate,
            PrivacyMode::default(),
        )
        .expect("draft");
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
        let mut gate = card_gate::evaluate("会议讨论阿里云方案", 1, 10_000, PrivacyMode::default())
            .expect("gate");
        gate.decision = GateDecision::Deny {
            reason: meeting_copilot_domain::model::gate::GateRejectReason::ConsentMissing,
            stopped_at: meeting_copilot_domain::model::gate::GateStage::G1,
            per_stage: meeting_copilot_domain::model::gate::PerStageResults::empty(),
            cooldown_until_ms: None,
        };
        assert!(build_draft(
            1,
            "阿里云",
            "会议讨论阿里云方案",
            &gate,
            PrivacyMode::default()
        )
        .is_none());
    }

    #[test]
    fn ingest_appends_cards_but_rejects_empty_text() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        let added = store.ingest("飞书 钉钉 腾讯会议").expect("added");
        assert!(added >= 1);
        assert!(!store.cards().is_empty());
        assert!(store.ingest("   ").is_err());
    }

    /// 界面上的「清空」：清掉本次会话累积的卡片，返回清掉的数量。
    #[test]
    fn clear_removes_all_cards_and_reports_count() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        let added = store.ingest("飞书 钉钉 腾讯会议").expect("added");
        assert!(added >= 1);
        assert_eq!(store.clear().expect("clear"), added);
        assert!(store.cards().is_empty());
        // 再清一次应为 0：清空是幂等的，不需要界面额外判重。
        assert_eq!(store.clear().expect("clear"), 0);
    }

    /// 自动路径同词冷却：同一句连说两遍，第二遍不再刷同一批卡片。
    #[test]
    fn auto_ingest_cools_down_repeated_terms() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        assert!(store.ingest("灰度发布 两地三中心 容量评估").expect("first") >= 1);
        assert_eq!(
            store
                .ingest("灰度发布 两地三中心 容量评估")
                .expect("second"),
            0,
            "同词冷却期内不应重复出卡"
        );
    }

    /// 手动入口不受限流：用户点了「生成卡片」就该出。
    #[test]
    fn manual_ingest_bypasses_throttle() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        assert!(
            store
                .ingest_manual("灰度发布 两地三中心 容量评估")
                .expect("first")
                >= 1
        );
        assert!(
            store
                .ingest_manual("灰度发布 两地三中心 容量评估")
                .expect("second")
                >= 1,
            "手动入口不应被冷却拦掉"
        );
    }

    /// 每分钟额度：连续自动 ingest 也不能无上限地刷卡片。
    #[test]
    fn auto_ingest_stops_at_per_minute_cap() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        let texts = [
            "灰度发布",
            "两地三中心",
            "容量评估",
            "链路压测",
            "回滚预案",
            "数据迁移",
            "风控规则",
            "结算对账",
            "指标口径",
            "巡检脚本",
            "备份恢复",
            "灰度回滚",
        ];
        let total: usize = texts
            .iter()
            .map(|text| store.ingest(text).expect("ingest"))
            .sum();
        assert!(total >= 1, "至少应出一张");
        assert!(
            total <= AUTO_CARDS_PER_MINUTE,
            "超出分钟额度：{total} > {AUTO_CARDS_PER_MINUTE}"
        );
    }

    /// 投屏时 G1 硬闸门直接拒绝（`prefilter.rs:76`），本地入口不出卡。
    #[test]
    fn screen_share_blocks_card_creation_at_g1() {
        let privacy = PrivacyMode {
            screen_share: true,
            emergency_masked: false,
        };
        let gate = card_gate::evaluate("会议讨论阿里云方案", 1, 10_000, privacy).expect("gate");
        assert!(matches!(
            &gate.decision,
            GateDecision::Deny {
                stopped_at: meeting_copilot_domain::model::gate::GateStage::G1,
                ..
            }
        ));
        let store = CardStore::new_with_privacy(privacy);
        assert_eq!(
            store.ingest("会议讨论阿里云方案").expect("ingest"),
            0,
            "投屏下不应产出卡片"
        );
    }

    /// 紧急遮蔽不拦 G1，但卡片必须不可点（`keyword.rs:186`）。
    #[test]
    fn emergency_mask_keeps_card_but_disables_click() {
        let privacy = PrivacyMode {
            screen_share: false,
            emergency_masked: true,
        };
        let gate = card_gate::evaluate("会议讨论阿里云方案", 1, 10_000, privacy).expect("gate");
        assert!(matches!(&gate.decision, GateDecision::Allow { .. }));
        let draft = build_draft(1, "阿里云", "会议讨论阿里云方案", &gate, privacy)
            .expect("遮蔽下仍出卡，只隐藏正文");
        assert_eq!(
            draft.click_state,
            meeting_copilot_domain::model::gate_keyword::ClickState::Disabled(
                meeting_copilot_domain::model::gate_keyword::ClickDisabledReason::EmergencyMasked
            )
        );
        assert!(draft.click_state_is_consistent());
        assert_eq!(draft.gate_decision_ref.trace_id(), &gate.trace_id);
    }
}
