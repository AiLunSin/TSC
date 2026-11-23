//! v2 迁移与主动提示仓储的持久化用例（SPEC 7.2、7.4，DESIGN 2.10.2）。

use meeting_copilot_domain::model::card::{DedupeKey, Reference};
use meeting_copilot_domain::model::enums::{Platform, TriggerKind};
use meeting_copilot_domain::model::feedback::{FeedbackEvent, FeedbackValue, TopicWeight};
use meeting_copilot_domain::model::proactive::{
    CachedQuery, OutboxEntry, OutboxState, CACHE_MAX_BYTES, CACHE_MAX_ENTRIES, CACHE_TTL_MS,
};
use meeting_copilot_domain::port::proactive::{FeedbackStore, OutboxStore, QueryCacheStore};
use meeting_copilot_domain::vo::{CardRef, ProviderId, SessionId, TaskId};
use meeting_copilot_infrastructure::store::{SqliteStore, StoreConfig, StoreKey};

/// `sqlcipher` feature 下文件库必须带密钥打开；固定密钥便于重启后仍能解密。
const TEST_DB_KEY: [u8; 32] = [42; 32];

fn open_store(config: &StoreConfig) -> SqliteStore {
    let key = StoreKey::new(TEST_DB_KEY);
    SqliteStore::open_with_key(config, Some(&key)).expect("open")
}

fn temp_config() -> StoreConfig {
    let path = std::env::temp_dir().join(format!(
        "tsc-proactive-{}-{:?}.db",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_file(&path);
    StoreConfig {
        path: path.to_string_lossy().into_owned(),
        key_ref: Some("keychain://tsc/test".to_owned()),
        wal_enabled: false,
    }
}

fn cache_entry(hash: &str, fetched_at_ms: i64, byte_size: usize) -> CachedQuery {
    CachedQuery::new(
        hash,
        ProviderId::new("provider-1").expect("provider"),
        None,
        fetched_at_ms,
        Vec::<Reference>::new(),
        byte_size,
        fetched_at_ms,
    )
    .expect("cache entry")
}

fn entry(seed: usize, state: OutboxState, created_at_ms: i64) -> OutboxEntry {
    let mut entry = OutboxEntry::new(
        TaskId::new(format!("task-{seed}")).expect("task"),
        None,
        TriggerKind::Auto,
        DedupeKey::new(format!("{seed:032x}")).expect("dedupe"),
        "topic-1",
        r#"{"kind":"metric"}"#,
        created_at_ms,
    )
    .expect("outbox entry");
    entry.state = state;
    entry
}

#[test]
fn cache_roundtrips_and_expires() {
    let store = SqliteStore::open_in_memory().expect("store");
    let cached = cache_entry(&"a".repeat(64), 0, 512);
    assert_eq!(QueryCacheStore::put(&store, &cached).expect("put"), 0);

    let loaded = QueryCacheStore::get(&store, &cached.query_hash)
        .expect("get")
        .expect("hit");
    assert_eq!(loaded, cached);
    assert!(loaded.is_usable(CACHE_TTL_MS - 1));
    assert!(!loaded.is_usable(CACHE_TTL_MS));

    assert_eq!(
        QueryCacheStore::sweep_expired(&store, CACHE_TTL_MS).expect("sweep"),
        1
    );
    assert!(QueryCacheStore::get(&store, &cached.query_hash)
        .expect("get")
        .is_none());
}

#[test]
fn cache_evicts_oldest_beyond_two_thousand_entries() {
    let store = SqliteStore::open_in_memory().expect("store");
    let mut evicted = 0;
    for index in 0..=CACHE_MAX_ENTRIES {
        let hash = format!("{index:064x}");
        evicted +=
            QueryCacheStore::put(&store, &cache_entry(&hash, index as i64, 64)).expect("put");
    }
    assert_eq!(evicted, 1);
    assert!(QueryCacheStore::get(&store, &format!("{:064x}", 0))
        .expect("get oldest")
        .is_none());
    assert!(
        QueryCacheStore::get(&store, &format!("{:064x}", CACHE_MAX_ENTRIES))
            .expect("get newest")
            .is_some()
    );
}

#[test]
fn cache_evicts_beyond_twenty_mib() {
    let store = SqliteStore::open_in_memory().expect("store");
    let chunk = 6 * 1024 * 1024;
    assert!(chunk <= CACHE_MAX_BYTES);
    let mut evicted = 0;
    for index in 0..4 {
        let hash = format!("{:064x}", index);
        evicted += QueryCacheStore::put(&store, &cache_entry(&hash, index, chunk)).expect("put");
    }
    assert_eq!(evicted, 1);
}

#[test]
fn outbox_survives_restart_with_same_active_set() {
    let config = temp_config();
    {
        let store = open_store(&config);
        OutboxStore::insert(&store, &entry(1, OutboxState::Pending, 10)).expect("pending");
        OutboxStore::insert(&store, &entry(2, OutboxState::Failed, 20)).expect("failed");
        OutboxStore::insert(&store, &entry(3, OutboxState::Done, 30)).expect("done");
        assert_eq!(store.active_count().expect("count"), 2);
    }
    {
        let store = open_store(&config);
        let recovered = OutboxStore::recoverable(&store).expect("recoverable");
        let ids: Vec<&str> = recovered
            .iter()
            .map(|entry| entry.task_id.as_str())
            .collect();
        assert_eq!(ids, vec!["task-1", "task-2"]);
        assert_eq!(store.schema_version().expect("version"), 2);
    }
    let _ = std::fs::remove_file(&config.path);
}

#[test]
fn migration_is_idempotent_and_preserves_existing_rows() {
    let config = temp_config();
    {
        let store = open_store(&config);
        let cached = cache_entry(&"b".repeat(64), 5, 128);
        QueryCacheStore::put(&store, &cached).expect("put");
    }
    {
        let store = open_store(&config);
        assert_eq!(store.schema_version().expect("version"), 2);
        assert!(QueryCacheStore::get(&store, &"b".repeat(64))
            .expect("get")
            .is_some());
    }
    let _ = std::fs::remove_file(&config.path);
}

#[test]
fn feedback_events_are_append_only_and_weights_roundtrip() {
    let store = SqliteStore::open_in_memory().expect("store");
    assert!(FeedbackStore::weight(&store, "topic-1", "")
        .expect("weight")
        .is_none());

    let event = FeedbackEvent {
        feedback_id: "fb-1".to_owned(),
        card_id: CardRef::new("card-1").expect("card"),
        dedupe_key: DedupeKey::new("c".repeat(32)).expect("dedupe"),
        topic_cluster_id: "topic-1".to_owned(),
        source_ref_id: None,
        trigger_kind: TriggerKind::Auto,
        platform: Platform::Zoom,
        value: FeedbackValue::Dismissed,
        occurred_at_ms: 1_000,
        weight_after_milli: 750,
    };
    FeedbackStore::append_event(&store, &event).expect("event");
    assert!(FeedbackStore::append_event(&store, &event).is_err());

    let weight = TopicWeight {
        topic_cluster_id: "topic-1".to_owned(),
        source_ref_id: String::new(),
        weight_milli: 750,
        blocked_until_ms: None,
        last_negative_at_ms: Some(1_000),
        updated_at_ms: 1_000,
    };
    FeedbackStore::save_weight(&store, &weight).expect("save");
    assert_eq!(
        FeedbackStore::weight(&store, "topic-1", "").expect("weight"),
        Some(weight)
    );
    assert_eq!(FeedbackStore::events(&store).expect("events").len(), 1);
}

#[test]
fn deleting_a_session_cascades_cache_and_outbox() {
    let store = SqliteStore::open_in_memory().expect("store");
    let session = SessionId::new("session-1").expect("session");
    let cached = CachedQuery::new(
        "d".repeat(64),
        ProviderId::new("provider-1").expect("provider"),
        Some(session.clone()),
        0,
        Vec::new(),
        64,
        0,
    )
    .expect("cache");
    QueryCacheStore::put(&store, &cached).expect("put");

    let mut queued = entry(1, OutboxState::Pending, 0);
    queued.session_id = Some(session.clone());
    OutboxStore::insert(&store, &queued).expect("insert");

    assert_eq!(
        QueryCacheStore::delete_session(&store, &session).expect("cache delete"),
        1
    );
    assert_eq!(
        OutboxStore::delete_session(&store, &session).expect("outbox delete"),
        1
    );
    assert_eq!(store.active_count().expect("count"), 0);
}

#[test]
fn empty_dedupe_lookup_returns_none() {
    let store = SqliteStore::open_in_memory().expect("store");
    let found = OutboxStore::find_active_by_dedupe(&store, &"e".repeat(32)).expect("lookup");
    assert!(found.is_none());
}
