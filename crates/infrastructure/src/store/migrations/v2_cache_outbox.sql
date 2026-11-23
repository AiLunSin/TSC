-- v2：检索缓存、断网 outbox、按需补全样本。
-- `session_id` 为可空扩展列，用于会话删除时级联清理（DESIGN 2.10.6）。

CREATE TABLE IF NOT EXISTS query_cache (
  query_hash          TEXT PRIMARY KEY,
  provider_profile_id TEXT NOT NULL,
  session_id          TEXT,
  fetched_at_ms       INTEGER NOT NULL,
  expires_at_ms       INTEGER NOT NULL,
  payload_refs_json   TEXT NOT NULL,
  byte_size           INTEGER NOT NULL CHECK (byte_size > 0),
  last_access_ms      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_query_cache_session ON query_cache(session_id);
CREATE INDEX IF NOT EXISTS idx_query_cache_access ON query_cache(last_access_ms);

CREATE TABLE IF NOT EXISTS proactive_outbox (
  task_id            TEXT PRIMARY KEY,
  session_id         TEXT,
  trigger_kind       TEXT NOT NULL,
  dedupe_key         TEXT NOT NULL,
  topic_cluster_id   TEXT NOT NULL,
  candidate_json     TEXT NOT NULL,
  state              TEXT NOT NULL CHECK (state IN ('pending', 'failed', 'dead', 'done')),
  attempts           INTEGER NOT NULL CHECK (attempts BETWEEN 0 AND 3),
  next_attempt_ms    INTEGER,
  created_at_ms      INTEGER NOT NULL,
  updated_at_ms      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_outbox_due ON proactive_outbox(state, next_attempt_ms);
CREATE INDEX IF NOT EXISTS idx_outbox_dedupe ON proactive_outbox(dedupe_key, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_outbox_session ON proactive_outbox(session_id);

CREATE TABLE IF NOT EXISTS on_demand_sample (
  sample_id           TEXT PRIMARY KEY,
  provider_profile_id TEXT NOT NULL,
  started_at_ms       INTEGER NOT NULL,
  latency_ms          INTEGER NOT NULL,
  outcome             TEXT NOT NULL CHECK (outcome IN ('success', 'timeout', 'failed', 'canceled')),
  cache_hit           INTEGER NOT NULL CHECK (cache_hit IN (0, 1))
);
CREATE INDEX IF NOT EXISTS idx_on_demand_provider ON on_demand_sample(provider_profile_id, started_at_ms);
