-- v5：卡片列表持久化（字段级加密），让重启后卡片仍可见。
-- 与 backfill_card 不同：本表是产品数据，跨重启保留；backfill_card 只服务补齐任务，完成即删。
CREATE TABLE IF NOT EXISTS card_snapshot (
  session_id TEXT NOT NULL,
  card_id    TEXT NOT NULL,
  payload    TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  PRIMARY KEY(session_id, card_id)
);
CREATE INDEX IF NOT EXISTS idx_card_snapshot_created ON card_snapshot(created_at_ms);