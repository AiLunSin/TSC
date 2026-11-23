-- v2：持久化审计哈希链、锚点与归档（SPEC 7.4、DESIGN 2.10.4）。
-- UPDATE 一律拒绝；DELETE 仅当记录序号落在已归档区间内时放行（DESIGN 2.10.6）。

CREATE TABLE IF NOT EXISTS audit_record (
  seq               INTEGER PRIMARY KEY AUTOINCREMENT,
  audit_id          TEXT NOT NULL UNIQUE,
  action            TEXT NOT NULL,
  actor             TEXT NOT NULL,
  occurred_at_ms    INTEGER NOT NULL,
  target            TEXT NOT NULL,
  sanitized_summary TEXT NOT NULL,
  prev_hash         TEXT NOT NULL,
  self_hash         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_record_occurred ON audit_record(occurred_at_ms);

CREATE TABLE IF NOT EXISTS audit_anchor (
  anchor_id      INTEGER PRIMARY KEY AUTOINCREMENT,
  up_to_seq      INTEGER NOT NULL,
  last_self_hash TEXT NOT NULL,
  anchor_hash    TEXT NOT NULL,
  created_at_ms  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_anchor_seq ON audit_anchor(up_to_seq);

CREATE TABLE IF NOT EXISTS audit_archive (
  month               TEXT PRIMARY KEY,
  first_seq           INTEGER NOT NULL,
  last_seq            INTEGER NOT NULL,
  record_count        INTEGER NOT NULL,
  terminal_self_hash  TEXT NOT NULL,
  bundle_path         TEXT NOT NULL,
  bundle_sha256       TEXT NOT NULL,
  sealed_at_ms        INTEGER NOT NULL,
  disposal_state      TEXT NOT NULL CHECK (disposal_state IN ('retained', 'pending_disposal'))
);

CREATE TRIGGER IF NOT EXISTS audit_record_no_update
BEFORE UPDATE ON audit_record
BEGIN
  SELECT RAISE(ABORT, 'audit append-only');
END;

CREATE TRIGGER IF NOT EXISTS audit_record_no_delete
BEFORE DELETE ON audit_record
WHEN NOT EXISTS (
  SELECT 1 FROM audit_archive
  WHERE month = strftime('%Y-%m', OLD.occurred_at_ms / 1000, 'unixepoch')
    AND OLD.seq BETWEEN first_seq AND last_seq
)
BEGIN
  SELECT RAISE(ABORT, 'audit append-only');
END;
