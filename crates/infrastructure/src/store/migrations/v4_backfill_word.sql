-- v4：断网补齐的查询词（字段级加密），让进程重启后仍能重建查询（SPEC 7.2.1-8a）。
-- 与 `proactive_outbox` 分开：outbox 本身不保存查询原文（7.2.1-11），
-- 这里只保留活跃任务所需的词，任务完成即删除，避免残留会议内容。
CREATE TABLE IF NOT EXISTS backfill_word (
  session_id TEXT NOT NULL,
  dedupe_key TEXT NOT NULL,
  payload    TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  PRIMARY KEY(session_id, dedupe_key)
);
CREATE INDEX IF NOT EXISTS idx_backfill_word_created ON backfill_word(created_at_ms);
-- 断网出卡时的最小卡片副本（字段级加密）：进程重启后据此重建卡片并补来源。
-- 只保留活跃任务对应的卡片，补齐完成即删除。
CREATE TABLE IF NOT EXISTS backfill_card (
  session_id TEXT NOT NULL,
  dedupe_key TEXT NOT NULL,
  payload    TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  PRIMARY KEY(session_id, dedupe_key)
);
