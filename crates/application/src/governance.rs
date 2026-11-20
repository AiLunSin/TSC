use std::sync::Mutex;

use meeting_copilot_domain::error::CopilotError;

/// 敏感操作类别。审计不可用时这四类操作一律拒绝（SPEC 5.6.3-4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveOp {
    Export,
    Upload,
    Delete,
    ConfigChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAvailability {
    Available,
    Degraded { backfilled: bool },
}

impl AuditAvailability {
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// 审计写入守卫。
///
/// 写失败后进入降级态：导出/上传/删除/配置变更被拒绝。
/// 恢复时自动补写缓冲并标记 `backfilled = true`。
pub struct AuditGuard {
    state: Mutex<GuardState>,
}

#[derive(Debug)]
struct GuardState {
    availability: AuditAvailability,
    buffered: Vec<String>,
}

impl AuditGuard {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(GuardState {
                availability: AuditAvailability::Available,
                buffered: Vec::new(),
            }),
        }
    }

    pub fn status(&self) -> AuditAvailability {
        self.lock().availability
    }

    pub fn check(&self, op: SensitiveOp) -> Result<(), CopilotError> {
        if self.lock().availability.is_available() {
            Ok(())
        } else {
            Err(CopilotError::PrivacyViolation {
                detail: format!("审计不可用，已拒绝敏感操作 {op:?}"),
            })
        }
    }

    /// 审计写入失败：缓冲本次摘要并进入降级态。
    pub fn on_audit_failure(&self, summary: impl Into<String>) {
        let mut state = self.lock();
        state.buffered.push(summary.into());
        state.availability = AuditAvailability::Degraded { backfilled: false };
    }

    /// 审计恢复：返回需要补写的缓冲，并将状态标记为已回填。
    pub fn on_audit_recovered(&self) -> Vec<String> {
        let mut state = self.lock();
        let pending = std::mem::take(&mut state.buffered);
        state.availability = AuditAvailability::Degraded { backfilled: true };
        pending
    }

    /// 补写完成后的显式确认。
    pub fn confirm_backfilled(&self) {
        self.lock().availability = AuditAvailability::Available;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, GuardState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for AuditGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sensitive_ops_blocked_when_degraded() {
        let guard = AuditGuard::new();
        for op in [
            SensitiveOp::Export,
            SensitiveOp::Upload,
            SensitiveOp::Delete,
            SensitiveOp::ConfigChange,
        ] {
            assert!(guard.check(op).is_ok());
        }

        guard.on_audit_failure("导出事件");
        for op in [
            SensitiveOp::Export,
            SensitiveOp::Upload,
            SensitiveOp::Delete,
            SensitiveOp::ConfigChange,
        ] {
            assert!(guard.check(op).is_err(), "{op:?} 必须被拒绝");
        }
    }

    #[test]
    fn recovery_returns_buffered_and_marks_backfilled() {
        let guard = AuditGuard::new();
        guard.on_audit_failure("事件 1");
        guard.on_audit_failure("事件 2");

        let pending = guard.on_audit_recovered();
        assert_eq!(pending, vec!["事件 1".to_owned(), "事件 2".to_owned()]);
        assert_eq!(
            guard.status(),
            AuditAvailability::Degraded { backfilled: true }
        );
        assert!(guard.check(SensitiveOp::Export).is_err());

        guard.confirm_backfilled();
        assert!(guard.check(SensitiveOp::Export).is_ok());
    }
}
