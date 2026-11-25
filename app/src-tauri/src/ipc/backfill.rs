//! 断网补齐桥：outbox 落库 + 网络恢复后自动回放补来源（SPEC 7.2.1-5/6/8/10）。
//!
//! 边界（如实记录，不夸大）：
//! - outbox 只落 `dedupe_key`、`topic_cluster_id` 与结构化候选，不保存查询文本（7.2.1-11）；
//!   回放所需的词来自本进程内的卡片标题索引，进程重启后任务仍在、但无法自动回放。
//! - 这里只负责落库、状态机与窗口统计；真实检索由 `SearchState` 注入执行。

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Mutex, MutexGuard};

use meeting_copilot_application::proactive::outbox::{EnqueueOutcome, OutboxService};
use meeting_copilot_application::proactive::recovery::{BACKFILL_RATIO_MIN, BACKFILL_WINDOW_MS};
use meeting_copilot_application::settings::{SecretBytes, SecureStore};
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::proactive::{OutboxEntry, BACKFILL_CONCURRENCY};
use meeting_copilot_domain::port::proactive::OutboxStore;
use meeting_copilot_domain::vo::{SessionId, TaskId};
use meeting_copilot_infrastructure::crypto::generate_master;
use meeting_copilot_infrastructure::store::repo::backfill_card::BackfillCardRecord;
use meeting_copilot_infrastructure::store::{SqliteStore, StoreConfig, StoreKey};
use meeting_copilot_platform_win::secure_store::{WindowsCredentialStore, DB_KEY_TARGET};
use sha2::{Digest, Sha256};

/// 补齐任务的去重键长度（`DedupeKey` 要求 32 位十六进制）。
const DEDUPE_HEX_LEN: usize = 32;
/// 补齐任务的话题簇标识。
const TOPIC_CLUSTER: &str = "desktop-search-source";
/// 桌面端单进程内会话标识；词表与 outbox 用同一个会话键。
const SESSION_ID: &str = "desktop-local-session";

/// 一次恢复窗口的取证结果；`meets_budget` 以「真正补上来源」的比例判定。
#[derive(Debug, Clone, PartialEq)]
pub struct BackfillReport {
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    pub elapsed_ms: i64,
    pub window_ms: i64,
    pub meets_budget: bool,
}

impl BackfillReport {
    pub fn ratio(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.done as f64 / self.total as f64
        }
    }
}

#[derive(Debug, Default)]
struct Window {
    start_ms: Option<i64>,
    total: usize,
    done: usize,
    failed: usize,
}

/// outbox 落库与恢复窗口统计。
pub struct BackfillBridge {
    store: SqliteStore,
    /// dedupe_key → 原始词；只在本进程内有效，用于回放时重建查询。
    words: Mutex<HashMap<String, String>>,
    window: Mutex<Window>,
    /// 任务 ID 唯一化：同一去重键可以反复失败/重试，主键不能复用。
    sequence: AtomicU64,
}

impl BackfillBridge {
    /// 打开受密钥保护的单库；密钥缺失或库打不开时返回 `None`，检索功能降级但不崩。
    pub fn open(config_dir: &Path) -> Option<Self> {
        let key = load_or_create_db_key()?;
        let config = StoreConfig {
            path: config_dir.join("meeting.db").to_string_lossy().into_owned(),
            key_ref: Some("dpapi://meeting-copilot/db".to_owned()),
            wal_enabled: true,
        };
        match SqliteStore::open_with_key(&config, Some(&StoreKey::new(key))) {
            Ok(store) => {
                let bridge = Self::from_store(store);
                // 重启后把活跃任务的查询词回灌到内存，并清掉已完成任务的残留。
                bridge.rehydrate();
                Some(bridge)
            }
            Err(error) => {
                tracing::warn!(target: "search", %error, "数据库打开失败，断网补齐停用");
                None
            }
        }
    }

    /// 进程重启后回灌活跃任务的查询词（词表加密落盘，SPEC 7.2.1-8a）。
    fn rehydrate(&self) {
        let Ok(entries) = self.store.recoverable() else {
            return;
        };
        let mut keep = Vec::with_capacity(entries.len());
        {
            let mut words = self.words();
            for entry in &entries {
                let dedupe = entry.dedupe_key.as_str().to_owned();
                keep.push(dedupe.clone());
                if let Ok(Some(word)) = self.store.load_backfill_word(SESSION_ID, &dedupe) {
                    words.insert(dedupe, word);
                }
            }
        }
        let _ = self.store.purge_backfill_words(SESSION_ID, &keep);
    }

    fn from_store(store: SqliteStore) -> Self {
        Self {
            store,
            words: Mutex::new(HashMap::new()),
            window: Mutex::new(Window::default()),
            sequence: AtomicU64::new(0),
        }
    }

    fn words(&self) -> MutexGuard<'_, HashMap<String, String>> {
        self.words
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn window(&self) -> MutexGuard<'_, Window> {
        self.window
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 先落库为 `pending`，再允许外发；同去重键合并到既有任务。
    pub fn enqueue(&self, word: &str, now_ms: i64) -> Option<TaskId> {
        let normalized = word.trim();
        if normalized.is_empty() {
            return None;
        }
        let dedupe = dedupe_of(normalized);
        // 查词的加密副本先落库：进程重启后靠它重建查询；落库失败则不外发。
        self.store
            .save_backfill_word(SESSION_ID, &dedupe, normalized, now_ms)
            .ok()?;
        let candidate_json = serde_json::json!({
            "kind": "card_source",
            "word_len": normalized.chars().count(),
            "word_hash": dedupe,
        })
        .to_string();
        // 任务 ID 不能只由去重键决定：同一个词可以反复失败/重试，主键必须是新的。
        let seq = self.sequence.fetch_add(1, AtomicOrdering::Relaxed);
        let task_id = format!("bf-{}-{now_ms}-{seq}", &dedupe[..16]);
        let entry = OutboxEntry::new(
            TaskId::new(task_id).ok()?,
            SessionId::new(SESSION_ID).ok(),
            TriggerKind::OnDemand,
            DedupeKey::new(dedupe.clone()).ok()?,
            TOPIC_CLUSTER,
            candidate_json,
            now_ms,
        )
        .ok()?;
        let outcome = OutboxService::new(&self.store).enqueue(entry).ok()?;
        let task_id = match outcome {
            EnqueueOutcome::Enqueued(id) | EnqueueOutcome::Merged(id) => id,
        };
        self.words().insert(dedupe, normalized.to_owned());
        Some(task_id)
    }

    /// 出卡自动补来源：在 `enqueue` 之外再落一份最小卡片副本，进程重启后据此重建卡片。
    pub fn enqueue_card(&self, word: &str, explanation: &str, now_ms: i64) -> Option<TaskId> {
        let task = self.enqueue(word, now_ms)?;
        let normalized = word.trim();
        let dedupe = dedupe_of(normalized);
        let record = BackfillCardRecord {
            card_id: format!("bfcard-{}", &dedupe[..16]),
            title: normalized.to_owned(),
            explanation: explanation.trim().to_owned(),
        };
        self.store
            .save_backfill_card(SESSION_ID, &dedupe, &record, now_ms)
            .ok()?;
        Some(task)
    }

    /// 重启后按词取回卡片副本；没有副本时返回 `None`。
    pub fn load_card(&self, word: &str) -> Option<BackfillCardRecord> {
        let dedupe = dedupe_of(word.trim());
        self.store
            .load_backfill_card(SESSION_ID, &dedupe)
            .ok()
            .flatten()
    }

    /// 按词结算对应的活跃任务：成功即 done，失败进入退避等待恢复。
    pub fn settle(&self, word: &str, ok: bool, now_ms: i64) {
        let normalized = word.trim();
        if normalized.is_empty() {
            return;
        }
        let dedupe = dedupe_of(normalized);
        let Ok(entries) = self.store.recoverable() else {
            return;
        };
        let service = OutboxService::new(&self.store);
        for mut entry in entries {
            if entry.dedupe_key.as_str() != dedupe {
                continue;
            }
            if ok {
                let _ = service.on_success(&mut entry, now_ms);
                // 任务完成即删词与卡片副本，避免会议内容残留。
                let _ = self.store.delete_backfill_word(SESSION_ID, &dedupe);
                let _ = self.store.delete_backfill_card(SESSION_ID, &dedupe);
                self.words().remove(&dedupe);
            } else {
                let _ = service.on_failure(&mut entry, now_ms);
            }
            return;
        }
    }

    /// 到期可回放的任务；同时带回重建查询所需的词（可能缺失）。
    pub fn due_jobs(&self, now_ms: i64) -> Vec<(OutboxEntry, Option<String>)> {
        let Ok(entries) = OutboxService::new(&self.store).due(now_ms) else {
            return Vec::new();
        };
        let words = self.words();
        entries
            .into_iter()
            .take(BACKFILL_CONCURRENCY)
            .map(|entry| {
                let dedupe = entry.dedupe_key.as_str();
                // 内存里没有时从加密词表兜底读：这是进程重启后仍能回放的关键。
                let word = words.get(dedupe).cloned().or_else(|| {
                    self.store
                        .load_backfill_word(SESSION_ID, dedupe)
                        .ok()
                        .flatten()
                });
                (entry, word)
            })
            .collect()
    }

    pub fn mark_done(&self, mut entry: OutboxEntry, now_ms: i64) {
        let dedupe = entry.dedupe_key.as_str().to_owned();
        let _ = OutboxService::new(&self.store).on_success(&mut entry, now_ms);
        let _ = self.store.delete_backfill_word(SESSION_ID, &dedupe);
        let _ = self.store.delete_backfill_card(SESSION_ID, &dedupe);
        self.words().remove(&dedupe);
    }

    pub fn mark_failed(&self, mut entry: OutboxEntry, now_ms: i64) {
        let _ = OutboxService::new(&self.store).on_failure(&mut entry, now_ms);
    }

    pub fn pending_total(&self) -> usize {
        self.store
            .recoverable()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    /// 网络恢复时开启补齐窗口。
    pub fn begin_window(&self, now_ms: i64, total: usize) {
        let mut window = self.window();
        window.start_ms = Some(now_ms);
        window.total = total;
        window.done = 0;
        window.failed = 0;
    }

    pub fn window_open(&self) -> bool {
        self.window().start_ms.is_some()
    }

    pub fn on_job_finished(&self, ok: bool) {
        let mut window = self.window();
        if window.start_ms.is_none() {
            return;
        }
        if ok {
            window.done = window.done.saturating_add(1);
        } else {
            window.failed = window.failed.saturating_add(1);
        }
    }

    /// 窗口关闭或到期时产出报告；比例按成功补齐数计算。
    pub fn poll_report(&self, now_ms: i64) -> Option<BackfillReport> {
        let mut window = self.window();
        let start = window.start_ms?;
        let elapsed = now_ms.saturating_sub(start);
        let processed = window.done.saturating_add(window.failed);
        let finished = processed >= window.total;
        if !finished && elapsed < BACKFILL_WINDOW_MS {
            return None;
        }
        let report = BackfillReport {
            total: window.total,
            done: window.done.min(window.total),
            failed: window.failed,
            elapsed_ms: elapsed,
            window_ms: BACKFILL_WINDOW_MS,
            meets_budget: (window.total == 0
                || window.done as f64 / window.total as f64 >= BACKFILL_RATIO_MIN)
                && elapsed <= BACKFILL_WINDOW_MS,
        };
        window.start_ms = None;
        window.total = 0;
        window.done = 0;
        window.failed = 0;
        Some(report)
    }
}

/// sha256 的前 16 字节十六进制：稳定、定长 32，满足 `DedupeKey`。
fn dedupe_of(word: &str) -> String {
    let digest = Sha256::digest(word.as_bytes());
    let mut out = String::with_capacity(DEDUPE_HEX_LEN);
    for byte in digest.iter().take(DEDUPE_HEX_LEN / 2) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// 读取 32 字节库密钥；首次运行生成并写入 OS 凭据库。读不到时不静默生成明文库。
fn load_or_create_db_key() -> Option<[u8; 32]> {
    const LEN: usize = 32;
    let store = WindowsCredentialStore::with_target_name(DB_KEY_TARGET);
    match store.read() {
        Ok(Some(bytes)) => {
            let raw = bytes.with_bytes(|raw| raw.to_vec());
            if raw.len() != LEN {
                tracing::warn!(target: "search", len = raw.len(), "数据库密钥长度非法，拒绝打开");
                return None;
            }
            let mut key = [0u8; LEN];
            key.copy_from_slice(&raw);
            Some(key)
        }
        Ok(None) => {
            let key = generate_master();
            store.write(&SecretBytes::new(key.to_vec())).ok()?;
            Some(key)
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge() -> BackfillBridge {
        BackfillBridge::from_store(SqliteStore::open_in_memory().expect("memory store"))
    }

    #[test]
    fn dedupe_is_stable_and_fixed_length() {
        let first = dedupe_of("灰度发布");
        assert_eq!(first.len(), DEDUPE_HEX_LEN);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(first, dedupe_of("灰度发布"));
        assert_ne!(first, dedupe_of("回滚预案"));
    }

    #[test]
    fn enqueue_persists_pending_and_merges_same_word() {
        let bridge = bridge();
        let now = 1_000;
        let first = bridge.enqueue("灰度发布", now).expect("enqueue");
        let again = bridge.enqueue("灰度发布", now).expect("merge");
        assert_eq!(first, again);
        assert_eq!(bridge.pending_total(), 1);
        let jobs = bridge.due_jobs(now);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].1.as_deref(), Some("灰度发布"));
    }

    #[test]
    fn missing_word_is_reported_without_claiming_success() {
        let bridge = bridge();
        // 直接插一条没有词表记录的任务（例如升级前的历史数据）。
        let entry = OutboxEntry::new(
            TaskId::new("bf-missing-1").expect("task id"),
            SessionId::new(SESSION_ID).ok(),
            TriggerKind::OnDemand,
            DedupeKey::new("a".repeat(DEDUPE_HEX_LEN)).expect("dedupe"),
            TOPIC_CLUSTER,
            serde_json::json!({ "kind": "card_source" }).to_string(),
            0,
        )
        .expect("entry");
        OutboxService::new(&bridge.store)
            .enqueue(entry)
            .expect("enqueue");
        let jobs = bridge.due_jobs(0);
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].1.is_none());
    }

    #[test]
    fn word_survives_lost_memory_and_done_clears_it() {
        let bridge = bridge();
        bridge.enqueue("灰度发布", 0).expect("enqueue");
        let dedupe = dedupe_of("灰度发布");
        assert_eq!(
            bridge
                .store
                .load_backfill_word(SESSION_ID, &dedupe)
                .expect("load word"),
            Some("灰度发布".to_owned())
        );
        // 模拟进程重启：内存词表为空，仍能从加密词表重建查询。
        bridge.words().clear();
        let (entry, word) = bridge.due_jobs(0).into_iter().next().expect("due job");
        assert_eq!(word.as_deref(), Some("灰度发布"));
        bridge.mark_done(entry, 1);
        assert!(bridge
            .store
            .load_backfill_word(SESSION_ID, &dedupe)
            .expect("load word")
            .is_none());
    }

    #[test]
    fn window_ratio_counts_only_successes() {
        let bridge = bridge();
        bridge.begin_window(0, 2);
        bridge.on_job_finished(true);
        bridge.on_job_finished(false);
        let report = bridge.poll_report(10).expect("closed");
        assert_eq!(report.done, 1);
        assert_eq!(report.failed, 1);
        assert!(!report.meets_budget);
    }

    #[test]
    fn repeated_word_gets_new_task_id_after_done() {
        let bridge = bridge();
        let first = bridge.enqueue("灰度发布", 0).expect("first");
        let entry = bridge.due_jobs(0).into_iter().next().expect("due job").0;
        bridge.mark_done(entry, 1);
        // 同一个词再次失败/重试时不能复用已完成的主键。
        let second = bridge.enqueue("灰度发布", 2).expect("second");
        assert_ne!(first, second);
        assert_eq!(bridge.pending_total(), 1);
    }
}
