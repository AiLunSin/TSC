//! 31.2 人工编辑锁定：锁定或已编辑的要点不得被自动改写。

use crate::model::enums::InsightStatus;
use crate::model::insight::Insight;

/// `manual_locked` 或 `Edited` 均视为锁定。
pub fn is_locked(insight: &Insight) -> bool {
    insight.manual_locked || insight.status == InsightStatus::Edited
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::enums::InsightKind;
    use crate::vo::{Confidence, InsightRef, ShortText, UtteranceRef, WallClock};

    fn insight() -> Insight {
        Insight::new(
            InsightRef::new("i1").expect("insight"),
            InsightKind::KeyInfo,
            ShortText::new("确认下周上线").expect("text"),
            Confidence::new(0.9).expect("conf"),
            vec![UtteranceRef::new("u1").expect("ref")],
            WallClock::from_unix_millis(0),
        )
        .expect("insight")
    }

    #[test]
    fn lock_state_is_explicit() {
        let mut item = insight();
        assert!(!is_locked(&item));
        item.manual_locked = true;
        assert!(is_locked(&item));
        item.manual_locked = false;
        item.status = InsightStatus::Edited;
        assert!(is_locked(&item));
    }
}
