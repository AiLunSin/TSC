-- v3：收紧审计删除触发器。仅已归档的 seq 区间可删，归档后新增的尾部记录不可删。

DROP TRIGGER IF EXISTS audit_record_no_delete;

CREATE TRIGGER audit_record_no_delete
BEFORE DELETE ON audit_record
WHEN NOT EXISTS (
  SELECT 1 FROM audit_archive
  WHERE month = strftime('%Y-%m', OLD.occurred_at_ms / 1000, 'unixepoch')
    AND OLD.seq BETWEEN first_seq AND last_seq
)
BEGIN
  SELECT RAISE(ABORT, 'audit append-only');
END;
