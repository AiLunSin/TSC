-- v6：卡片质量事件，让"自动展示 / 前端可见 / 用户反馈"有可测出口。
-- 只存卡 ID、事件枚举、来源、时间与数值，不存转写正文或关键词原文。

CREATE TABLE IF NOT EXISTS quality_event (
  event_id   TEXT PRIMARY KEY,
  event_kind TEXT NOT NULL CHECK (event_kind IN ('auto_shown', 'rendered', 'feedback')),
  card_id    TEXT NOT NULL,
  source     TEXT NOT NULL CHECK (source IN ('auto', 'manual')),
  at_ms      INTEGER NOT NULL,
  latency_ms INTEGER NOT NULL DEFAULT 0,
  outcome    TEXT CHECK (outcome IN ('useful', 'useless', 'wrong'))
);
CREATE INDEX IF NOT EXISTS idx_quality_kind_at ON quality_event(event_kind, at_ms);
CREATE INDEX IF NOT EXISTS idx_quality_card ON quality_event(card_id);