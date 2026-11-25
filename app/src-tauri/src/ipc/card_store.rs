//! 卡片生成：把一段会议文本提取为关键词，并快速产出可点击卡片。
//!
//! 来源为 `UserGenerated`（本地会议片段），因此卡片可合法进入可点击态；
//! 不伪造外部检索来源。

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
use meeting_copilot_infrastructure::store::repo::card_snapshot::StoredCard;
use meeting_copilot_infrastructure::store::repo::quality::{
    QualityEvent, QualityMetrics, OUTCOME_USEFUL, OUTCOME_USELESS, OUTCOME_WRONG, SOURCE_AUTO,
    SOURCE_MANUAL,
};
use meeting_copilot_infrastructure::store::SqliteStore;

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
/// 卡片持久化与补齐共用同一个桌面会话标识。
const CARD_SESSION_ID: &str = "desktop-local-session";

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
    /// 卡片持久化：出卡落库、启动回灌、清空同步删库；测试可无库。
    store: Option<Arc<SqliteStore>>,
    /// 回灌期间不再写库，避免重复插入。
    restoring: AtomicBool,
}

impl CardStore {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_store(None, PrivacyMode::from_env())
    }

    /// 带持久化：重启后卡片仍可见（SPEC 4.3-3 单库；字段级加密）。
    pub(crate) fn new_persistent(store: Option<Arc<SqliteStore>>) -> Self {
        Self::with_store(store, PrivacyMode::from_env())
    }

    /// 显式给定隐私态：测试用固定值，生产由持久化构造从开发期开关读取。
    #[cfg(test)]
    pub(crate) fn new_with_privacy(privacy: PrivacyMode) -> Self {
        Self::with_store(None, privacy)
    }

    fn with_store(store: Option<Arc<SqliteStore>>, privacy: PrivacyMode) -> Self {
        Self {
            cards: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
            jieba: Jieba::new(),
            tfidf: TfIdf::new(None::<&mut &[u8]>, KeywordExtractConfig::default()),
            privacy,
            auto: Mutex::new(AutoEmitState::default()),
            store,
            restoring: AtomicBool::new(false),
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
        let started = Instant::now();
        // 转写文本可能带 vosk 的词间空格，抽词和摘要前统一合上。
        let text = crate::ipc::transcript::tidy_cjk_spaces(text.trim());
        let text = text.as_str();
        let keywords = self.extract_keywords(text);
        if keywords.is_empty() {
            return Err(IpcErrorDto::new(
                "no_keyword",
                "未从文本中识别到关键词。",
                false,
            ));
        }
        // 多留一些原料，摘要再按句子边界截到 80 字（见 `fit_text`）。
        let excerpt: String = text.chars().take(200).collect();
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
        let mut shown: Vec<(String, u64)> = Vec::new();
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
            match build_draft(sequence, &keyword, &excerpt, &gate, self.privacy, &[]) {
                Some(draft) => {
                    let card_id = draft.card_id.as_str().to_owned();
                    store.push(CardSnapshotEntry::Draft(draft));
                    added += 1;
                    if limited {
                        emitted.push(keyword.clone());
                        shown.push((card_id, now));
                    }
                }
                None => continue,
            }
        }
        drop(store);
        // 只给真正出过卡的词记冷却，避免"被门控拦掉的词也算用掉额度"。
        let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        for (card_id, at_ms) in shown {
            self.record_shown(&card_id, at_ms, latency_ms);
        }
        for term in emitted {
            self.record_auto(&term, now);
        }
        Ok(added)
    }

    /// 自动路径出卡时写质量事件；无库（测试/降级）时静默跳过。
    fn record_shown(&self, card_id: &str, at_ms: u64, latency_ms: u64) {
        let Some(store) = &self.store else {
            return;
        };
        let _ = store.record_quality_event(&QualityEvent::shown(card_id, at_ms, latency_ms));
    }

    /// 记录用户反馈；`wrong` 同时撤回当前卡。返回是否发生撤回。
    pub fn record_feedback(&self, card_id: &str, outcome: &str) -> Result<bool, IpcErrorDto> {
        if card_id.trim().is_empty() {
            return Err(IpcErrorDto::new("invalid_feedback", "卡片不存在。", false));
        }
        if !matches!(outcome, OUTCOME_USEFUL | OUTCOME_USELESS | OUTCOME_WRONG) {
            return Err(IpcErrorDto::new(
                "invalid_feedback",
                "反馈取值不合法。",
                false,
            ));
        }
        let source = match &self.store {
            Some(store) if store.has_auto_shown(card_id) => SOURCE_AUTO,
            _ => SOURCE_MANUAL,
        };
        if let Some(store) = &self.store {
            let _ = store.record_quality_event(&QualityEvent::feedback(
                card_id,
                outcome,
                now_ms(),
                source,
            ));
        }
        let withdrew = outcome == OUTCOME_WRONG && self.withdraw(card_id);
        Ok(withdrew)
    }

    /// 前端第一次看到这张自动卡时上报；同卡只保留最新一条可见时延。
    pub fn mark_rendered(&self, card_id: &str) -> Result<(), IpcErrorDto> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        let Some(shown_at) = store.auto_shown_at(card_id) else {
            return Ok(());
        };
        let now = now_ms();
        let shown = u64::try_from(shown_at).unwrap_or(now);
        // 历史卡回灌后首次加载也会出现在列表里；超过 5 分钟不算"本轮可见时延"。
        if now.saturating_sub(shown) > 5 * 60 * 1000 {
            return Ok(());
        }
        let latency_ms = now.saturating_sub(shown);
        let _ = store.record_quality_event(&QualityEvent::rendered(
            card_id,
            now,
            latency_ms,
            SOURCE_AUTO,
        ));
        Ok(())
    }

    /// 质量统计；`hours=0` 表示不限窗口。无库时返回空指标（界面显示样本不足）。
    pub fn quality_metrics(&self, hours: Option<u32>) -> Result<QualityMetrics, IpcErrorDto> {
        let hours = hours.unwrap_or(0).min(24 * 30);
        let since = if hours == 0 {
            0
        } else {
            now_ms().saturating_sub(u64::from(hours) * 3_600_000)
        };
        let since = i64::try_from(since).unwrap_or(i64::MAX);
        match &self.store {
            Some(store) => store
                .quality_metrics(since)
                .map_err(|_| IpcErrorDto::new("quality_unavailable", "质量统计暂不可用。", true)),
            None => Ok(QualityMetrics::default()),
        }
    }

    /// 从内存与持久化副本同时撤掉一张卡。
    fn withdraw(&self, card_id: &str) -> bool {
        let removed = self
            .cards
            .lock()
            .map(|mut cards| {
                let before = cards.len();
                cards.retain(|entry| match entry {
                    CardSnapshotEntry::Draft(draft) => draft.card_id.as_str() != card_id,
                    _ => true,
                });
                before != cards.len()
            })
            .unwrap_or(false);
        if let Some(store) = &self.store {
            let _ = store.delete_card_snapshot(CARD_SESSION_ID, card_id);
        }
        removed
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
    /// 不带来源的便捷入口；生产路径走 [`Self::ingest_explanation_with_sources`]。
    #[cfg(test)]
    pub fn ingest_explanation(&self, word: &str, explanation: &str) -> Result<usize, IpcErrorDto> {
        self.ingest_explanation_with_sources(word, explanation, &[])
    }

    /// 带来源的释义出卡：`sources` 为联网检索到的站点，非空时引用标为 `GeneralWeb`。
    ///
    /// 来源只影响"这张卡的出处"，不改变门控与隐私判定；来源不合法时退回本机引用，
    /// 不会出现"标注了外部来源却没有引用"的卡片。
    pub fn ingest_explanation_with_sources(
        &self,
        word: &str,
        explanation: &str,
        sources: &[String],
    ) -> Result<usize, IpcErrorDto> {
        // 选区可能来自带空格的转写行，标题也要合上，否则卡片标题是"学历 歧视"。
        let word = crate::ipc::transcript::tidy_cjk_spaces(word.trim());
        let word = word.as_str();
        let explanation = crate::ipc::transcript::tidy_cjk_spaces(explanation.trim());
        let explanation = explanation.as_str();
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
        // 释义最长 180 字，这里留全量，由 `build_draft` 按句子边界截到卡片上限。
        let excerpt: String = explanation.chars().take(300).collect();
        let mut store = self
            .cards
            .lock()
            .map_err(|_| IpcErrorDto::new("card_store_unavailable", "卡片存储不可用。", true))?;
        if store.len() >= MAX_CARDS {
            store.remove(0);
        }
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        match build_draft(sequence, &title, &excerpt, &gate, self.privacy, sources) {
            Some(draft) => {
                self.persist(&draft);
                store.push(CardSnapshotEntry::Draft(draft));
                Ok(1)
            }
            None => Ok(0),
        }
    }

    /// 把补齐检索到的站点写回已生成的卡片；找不到卡片或来源为空时返回 `false`。
    ///
    /// 只补来源，不改写释义与门控结论；用于断网恢复后的自动回放（SPEC 7.2.1-8/10）。
    pub fn attach_sources(&self, word: &str, sources: &[String]) -> bool {
        if sources.is_empty() {
            return false;
        }
        let word = crate::ipc::transcript::tidy_cjk_spaces(word.trim());
        let Ok(mut store) = self.cards.lock() else {
            return false;
        };
        let sequence = self.sequence.load(Ordering::Relaxed);
        for entry in store.iter_mut().rev() {
            let CardSnapshotEntry::Draft(draft) = entry else {
                continue;
            };
            if draft.keyword_text.as_str() != word {
                continue;
            }
            // 已经补过网页来源的卡不再重复补；同词多张断网卡按“最近一张未补”依次补齐。
            let already_web = draft
                .references
                .iter()
                .any(|reference| reference.trust_tier == ReferenceTrustTier::GeneralWeb);
            if already_web {
                continue;
            }
            draft.references = build_references(sequence, sources);
            draft.citation_alignment = CitationAlignment::Passed;
            // 来源变了，落库副本同步更新。
            self.persist(&draft.clone());
            return true;
        }
        false
    }

    /// 出卡/补来源后把卡片落库（字段级加密）；重启后用 `restore` 回灌。
    fn persist(&self, draft: &KeywordCardDraft) {
        let Some(store) = &self.store else {
            return;
        };
        if self.restoring.load(Ordering::Relaxed) {
            return;
        }
        let sources: Vec<String> = draft
            .references
            .iter()
            .filter(|reference| reference.trust_tier == ReferenceTrustTier::GeneralWeb)
            .filter_map(|reference| {
                reference
                    .reference_id
                    .as_str()
                    .split_once(':')
                    .map(|(_, site)| site.to_owned())
            })
            .collect();
        let record = StoredCard {
            card_id: draft.card_id.as_str().to_owned(),
            title: draft.keyword_text.as_str().to_owned(),
            explanation: draft
                .summary
                .as_ref()
                .map(|summary| summary.as_str().to_owned())
                .unwrap_or_default(),
            sources,
            created_at_ms: i64::try_from(now_ms()).unwrap_or(i64::MAX),
        };
        let _ = store.save_card_snapshot(CARD_SESSION_ID, &record);
    }

    /// 启动时回灌最近卡片；回灌期间不重复写库。
    pub fn restore(&self) {
        let Some(store) = &self.store else {
            return;
        };
        let Ok(records) = store.load_recent_cards(CARD_SESSION_ID, MAX_CARDS) else {
            return;
        };
        if records.is_empty() {
            return;
        }
        self.restoring.store(true, Ordering::Relaxed);
        for record in records {
            let _ = self.ingest_explanation_with_sources(
                &record.title,
                &record.explanation,
                &record.sources,
            );
        }
        self.restoring.store(false, Ordering::Relaxed);
    }

    /// 清空本次会话的卡片，供界面上的"清空"按钮使用。
    pub fn clear(&self) -> Result<usize, IpcErrorDto> {
        let mut store = self
            .cards
            .lock()
            .map_err(|_| IpcErrorDto::new("card_store_unavailable", "卡片存储不可用。", true))?;
        let removed = store.len();
        store.clear();
        drop(store);
        if let Some(sqlite) = &self.store {
            let _ = sqlite.delete_card_snapshots(CARD_SESSION_ID);
        }
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
    sources: &[String],
) -> Option<KeywordCardDraft> {
    let title: String = keyword.chars().take(TitleText::LIMIT).collect();
    let summary = fit_text(excerpt, ConclusionText::LIMIT);
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
        references: build_references(sequence, sources),
        citation_alignment: CitationAlignment::Passed,
        gate_decision_ref: admission.allow_ref?,
        card_status: KeywordCardStatus::Displayed,
        click_state: admission.click_state,
    })
}

/// 卡片引用：有网络来源就按来源逐条标 `GeneralWeb`，没有才用本机片段。
///
/// `reference_id` 里带站点名（`web-0:example.com`），卡片详情能直接看出出处；
/// 站点串不合法时跳过该条，全部跳过则退回本机引用。
fn build_references(sequence: u64, sources: &[String]) -> Vec<ReferenceView> {
    let mut references: Vec<ReferenceView> = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let site = source.trim();
        if site.is_empty() {
            continue;
        }
        let Ok(reference_id) = ReferenceRef::new(format!("web-{index}:{site}")) else {
            continue;
        };
        references.push(ReferenceView {
            reference_id,
            trust_tier: ReferenceTrustTier::GeneralWeb,
            access_status: ReferenceAccessStatus::Available,
            published_at_ms: None,
            alignment: CitationAlignment::Passed,
        });
    }
    if references.is_empty() {
        if let Ok(reference_id) = ReferenceRef::new(format!("local-{sequence}")) {
            references.push(ReferenceView {
                reference_id,
                trust_tier: ReferenceTrustTier::UserGenerated,
                access_status: ReferenceAccessStatus::Available,
                published_at_ms: None,
                alignment: CitationAlignment::Passed,
            });
        }
    }
    references
}

/// 把长文本截到卡片上限以内，并尽量停在句子/分句边界上。
///
/// 直接 `take(80)` 会把英文词切成 `educational c` 这种残片，看着像故障。
/// 优先在 `。！？；，` 处收口并补省略号；找不到断点才硬截。
fn fit_text(text: &str, limit: usize) -> String {
    let chars: Vec<char> = text.trim().chars().collect();
    if chars.len() <= limit {
        return chars.into_iter().collect();
    }
    const BREAKS: [char; 7] = ['。', '！', '？', '；', '，', '!', ','];
    if let Some(position) = chars[..limit].iter().rposition(|c| BREAKS.contains(c)) {
        let mut out: String = chars[..=position].iter().collect();
        // 断点本身可能正好贴着上限，补省略号会超一位，这时就不补。
        if position + 2 <= limit {
            out.push('…');
        }
        return out;
    }
    let mut out: String = chars[..limit - 1].iter().collect();
    out.push('…');
    out
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
            &[],
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
            PrivacyMode::default(),
            &[]
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
        let draft = build_draft(1, "阿里云", "会议讨论阿里云方案", &gate, privacy, &[])
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

    /// 摘要不能把英文词切成 `educational c` 这种残片。
    #[test]
    fn summary_truncates_on_sentence_boundary() {
        let long = "这里的学历歧视指招聘时以学历区别对待求职者。它并非严格专业术语，通常对应英文 degree discrimination 或 \
                    educational credential discrimination。上下文用它引出个案背后的普遍问题。";
        let summary = fit_text(long, 80);
        assert!(summary.chars().count() <= 80, "{summary}");
        assert!(summary.ends_with('…'), "{summary}");
        assert!(!summary.contains("educational c…"), "{summary}");
        // 短文本原样返回，不补省略号。
        assert_eq!(fit_text("一句话结论。", 80), "一句话结论。");
    }

    /// 从带空格的转写行选中文字出卡，标题也要合上空格。
    #[test]
    fn explanation_title_merges_cjk_spaces() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        let added = store
            .ingest_explanation("学历 歧视", "指招聘时以学历高低区别对待求职者。")
            .expect("added");
        assert_eq!(added, 1);
        let cards = store.cards();
        let CardSnapshotEntry::Draft(draft) = &cards[0] else {
            panic!("应为草案卡");
        };
        assert_eq!(draft.keyword_text.as_str(), "学历歧视");
    }

    /// 带网络来源时，引用逐条标 `GeneralWeb`，且站点写在引用 ID 里。
    #[test]
    fn explanation_with_sources_marks_web_references() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        let added = store
            .ingest_explanation_with_sources(
                "SLA",
                "服务等级协议，约定可用性与响应时限。",
                &["example.com".to_owned(), "ops.example.org".to_owned()],
            )
            .expect("added");
        assert_eq!(added, 1);
        let cards = store.cards();
        let CardSnapshotEntry::Draft(draft) = &cards[0] else {
            panic!("应为草案卡");
        };
        assert_eq!(draft.references.len(), 2);
        assert_eq!(
            draft.references[0].trust_tier,
            ReferenceTrustTier::GeneralWeb
        );
        assert_eq!(
            draft.references[0].reference_id.as_str(),
            "web-0:example.com"
        );
    }

    /// 来源串非法或为空时退回本机引用，不能出现"标了外部来源却没有引用"。
    #[test]
    fn blank_sources_fall_back_to_local_reference() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        let added = store
            .ingest_explanation_with_sources("灰度", "分批次放量。", &["   ".to_owned()])
            .expect("added");
        assert_eq!(added, 1);
        let cards = store.cards();
        let CardSnapshotEntry::Draft(draft) = &cards[0] else {
            panic!("应为草案卡");
        };
        assert_eq!(draft.references.len(), 1);
        assert_eq!(
            draft.references[0].trust_tier,
            ReferenceTrustTier::UserGenerated
        );
    }

    /// 同词多张断网卡按“最近一张未补”依次补齐，已补过的卡不重复补。
    #[test]
    fn attach_sources_fills_each_offline_card_once() {
        let store = CardStore::new_with_privacy(PrivacyMode::default());
        for _ in 0..2 {
            store
                .ingest_explanation_with_sources("灰度", "分批次放量。", &[])
                .expect("card");
        }
        assert!(store.attach_sources("灰度", &["example.com".to_owned()]));
        assert!(store.attach_sources("灰度", &["ops.example.org".to_owned()]));
        assert!(!store.attach_sources("灰度", &["third.example.net".to_owned()]));
        let cards = store.cards();
        assert_eq!(cards.len(), 2);
        for card in &cards {
            let CardSnapshotEntry::Draft(draft) = card else {
                panic!("应为草案卡");
            };
            assert_eq!(
                draft.references[0].trust_tier,
                ReferenceTrustTier::GeneralWeb
            );
        }
    }

    /// 卡片落库后，重启（新建 Store 实例）仍能回灌；清空同步删库。
    #[test]
    fn cards_survive_restart_through_store() {
        let sqlite = std::sync::Arc::new(
            meeting_copilot_infrastructure::store::SqliteStore::open_in_memory().expect("store"),
        );
        let first = CardStore::new_persistent(Some(std::sync::Arc::clone(&sqlite)));
        first
            .ingest_explanation_with_sources("灰度", "分批次放量。", &["example.com".to_owned()])
            .expect("card");
        assert_eq!(
            sqlite
                .load_recent_cards("desktop-local-session", 10)
                .expect("load")
                .len(),
            1
        );

        let second = CardStore::new_persistent(Some(std::sync::Arc::clone(&sqlite)));
        second.restore();
        assert_eq!(second.cards().len(), 1);
        second.clear().expect("clear");
        assert!(sqlite
            .load_recent_cards("desktop-local-session", 10)
            .expect("load")
            .is_empty());
    }

    /// 质量链路：自动出卡写 shown，前端可见写 rendered，"错误"反馈撤回卡片。
    #[test]
    fn quality_events_are_recorded_and_wrong_feedback_withdraws() {
        let sqlite = std::sync::Arc::new(SqliteStore::open_in_memory().expect("store"));
        let cards = CardStore::new_persistent(Some(std::sync::Arc::clone(&sqlite)));
        let added = cards
            .ingest("灰度发布下周三开始，由张三负责，先完成监控告警演练，并确认回滚方案。")
            .expect("ingest");
        assert!(added >= 1);

        let card_id = cards
            .cards()
            .iter()
            .find_map(|entry| match entry {
                CardSnapshotEntry::Draft(draft) => Some(draft.card_id.as_str().to_owned()),
                _ => None,
            })
            .expect("card");
        cards.mark_rendered(&card_id).expect("rendered");
        let withdrawn = cards
            .record_feedback(&card_id, OUTCOME_WRONG)
            .expect("feedback");
        assert!(withdrawn, "错误反馈应撤回当前卡");
        assert!(!cards.cards().iter().any(|entry| matches!(
            entry,
            CardSnapshotEntry::Draft(draft) if draft.card_id.as_str() == card_id
        )));
        assert!(sqlite
            .load_recent_cards("desktop-local-session", 10)
            .expect("load")
            .iter()
            .all(|record| record.card_id != card_id));

        let metrics = cards.quality_metrics(Some(24)).expect("metrics");
        assert!(metrics.auto_shown >= 1);
        assert_eq!(metrics.feedback, 1);
        assert_eq!(metrics.wrong, 1);
        assert_eq!(metrics.mis_trigger_rate, Some(1.0));
        assert_eq!(metrics.latency_samples, 1);
    }

    /// 手动卡不进误触发率分母，也不记可见时延。
    #[test]
    fn manual_card_feedback_is_excluded_from_auto_rates() {
        let sqlite = std::sync::Arc::new(SqliteStore::open_in_memory().expect("store"));
        let cards = CardStore::new_persistent(Some(std::sync::Arc::clone(&sqlite)));
        cards
            .ingest_explanation_with_sources("灰度", "分批次放量。", &[])
            .expect("card");
        let card_id = cards
            .cards()
            .iter()
            .find_map(|entry| match entry {
                CardSnapshotEntry::Draft(draft) => Some(draft.card_id.as_str().to_owned()),
                _ => None,
            })
            .expect("card");
        cards.mark_rendered(&card_id).expect("rendered");
        cards
            .record_feedback(&card_id, OUTCOME_USELESS)
            .expect("feedback");

        let metrics = cards.quality_metrics(Some(24)).expect("metrics");
        assert_eq!(metrics.auto_shown, 0);
        assert_eq!(metrics.feedback, 0);
        assert_eq!(metrics.mis_trigger_rate, None);
        assert_eq!(metrics.latency_samples, 0);
    }
}
