-- v2：负反馈事件与派生主题权重（SPEC 7.1.1-8 字段白名单）。

CREATE TABLE IF NOT EXISTS feedback_event (
  feedback_id        TEXT PRIMARY KEY,
  card_id            TEXT NOT NULL,
  dedupe_key         TEXT NOT NULL,
  topic_cluster_id   TEXT NOT NULL,
  source_ref_id      TEXT,
  trigger_kind       TEXT NOT NULL,
  platform           TEXT NOT NULL,
  value              TEXT NOT NULL CHECK (value IN ('adopted', 'dismissed', 'reported')),
  occurred_at_ms     INTEGER NOT NULL,
  weight_after_milli INTEGER NOT NULL CHECK (weight_after_milli BETWEEN 100 AND 1000)
);
CREATE INDEX IF NOT EXISTS idx_feedback_topic ON feedback_event(topic_cluster_id, occurred_at_ms);

CREATE TABLE IF NOT EXISTS topic_weight (
  topic_cluster_id   TEXT NOT NULL,
  source_ref_id      TEXT NOT NULL DEFAULT '',
  weight_milli       INTEGER NOT NULL CHECK (weight_milli BETWEEN 100 AND 1000),
  blocked_until_ms   INTEGER,
  last_negative_at_ms INTEGER,
  updated_at_ms      INTEGER NOT NULL,
  PRIMARY KEY (topic_cluster_id, source_ref_id)
);
