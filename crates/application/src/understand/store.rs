//! 内存要点集合与 `ref → session` 映射（D-02）。

use std::collections::HashMap;
use std::sync::Mutex;

use meeting_copilot_domain::model::enums::InsightStatus;
use meeting_copilot_domain::model::insight::Insight;
use meeting_copilot_domain::vo::{InsightRef, SessionId, UtteranceRef};

#[derive(Default)]
struct Inner {
    insights: Vec<Insight>,
    ref_session: HashMap<UtteranceRef, SessionId>,
}

/// 会话内要点集合：插入不覆盖既有引用，删除只改状态。
#[derive(Default)]
pub struct InsightStore {
    inner: Mutex<Inner>,
}

impl InsightStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    pub fn register_refs(&self, refs: &[UtteranceRef], session: &SessionId) {
        let mut inner = self.lock();
        for reference in refs {
            inner
                .ref_session
                .entry(reference.clone())
                .or_insert_with(|| session.clone());
        }
    }

    pub fn session_of(&self, reference: &UtteranceRef) -> Option<SessionId> {
        self.lock().ref_session.get(reference).cloned()
    }

    pub fn within(&self, session: &SessionId) -> Vec<Insight> {
        let inner = self.lock();
        inner
            .insights
            .iter()
            .filter(|insight| {
                insight
                    .sources
                    .iter()
                    .any(|reference| inner.ref_session.get(reference) == Some(session))
            })
            .cloned()
            .collect()
    }

    pub fn visible(&self, session: &SessionId) -> Vec<Insight> {
        self.within(session)
            .into_iter()
            .filter(|insight| insight.status != InsightStatus::Deleted)
            .collect()
    }

    /// 同 `reference` 已存在时不覆盖，返回 `false`。
    pub fn insert(&self, insight: Insight) -> bool {
        let mut inner = self.lock();
        if inner
            .insights
            .iter()
            .any(|existing| existing.reference == insight.reference)
        {
            return false;
        }
        inner.insights.push(insight);
        true
    }

    pub fn update<F: FnOnce(&mut Insight)>(&self, reference: &InsightRef, edit: F) -> bool {
        let mut inner = self.lock();
        match inner
            .insights
            .iter_mut()
            .find(|insight| insight.reference == *reference)
        {
            Some(insight) => {
                edit(insight);
                true
            }
            None => false,
        }
    }

    pub fn get(&self, reference: &InsightRef) -> Option<Insight> {
        self.lock()
            .insights
            .iter()
            .find(|insight| insight.reference == *reference)
            .cloned()
    }

    pub fn all(&self) -> Vec<Insight> {
        self.lock().insights.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::enums::InsightKind;
    use meeting_copilot_domain::vo::{Confidence, ShortText, WallClock};

    fn insight(reference: &str, source: &str) -> Insight {
        Insight::new(
            InsightRef::new(reference).expect("insight"),
            InsightKind::KeyInfo,
            ShortText::new("确认下周上线").expect("text"),
            Confidence::new(0.9).expect("conf"),
            vec![UtteranceRef::new(source).expect("ref")],
            WallClock::from_unix_millis(0),
        )
        .expect("insight")
    }

    #[test]
    fn store_is_session_scoped() {
        let store = InsightStore::default();
        let s1 = SessionId::new("s1").expect("session");
        let s2 = SessionId::new("s2").expect("session");
        store.register_refs(&[UtteranceRef::new("u1").expect("ref")], &s1);
        store.register_refs(&[UtteranceRef::new("u2").expect("ref")], &s2);
        assert!(store.insert(insight("i1", "u1")));
        assert!(store.insert(insight("i2", "u2")));
        assert_eq!(store.within(&s1).len(), 1);
        assert_eq!(store.within(&s2).len(), 1);
        assert_eq!(
            store.session_of(&UtteranceRef::new("u1").expect("ref")),
            Some(s1)
        );
    }

    #[test]
    fn store_does_not_overwrite_existing_reference() {
        let store = InsightStore::default();
        assert!(store.insert(insight("i1", "u1")));
        assert!(!store.insert(insight("i1", "u2")));
        assert_eq!(store.all().len(), 1);
    }

    #[test]
    fn store_update_by_reference() {
        let store = InsightStore::default();
        store.insert(insight("i1", "u1"));
        let reference = InsightRef::new("i1").expect("insight");
        assert!(store.update(&reference, |insight| {
            insight.flagged_important = true;
        }));
        assert!(store.all()[0].flagged_important);
        assert!(!store.update(&InsightRef::new("i9").expect("insight"), |_| {}));
    }
}
