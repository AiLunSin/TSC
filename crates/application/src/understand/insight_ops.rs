//! 要点标记 / 删除 / 筛选；三处消费者统一走 `is_visible`。

use meeting_copilot_domain::model::enums::{InsightKind, InsightStatus};
use meeting_copilot_domain::model::insight::Insight;
use meeting_copilot_domain::vo::{InsightRef, SessionId};

use super::deduplicator::is_visible;
use super::store::InsightStore;

pub fn flag_important(store: &InsightStore, reference: &InsightRef, flag: bool) -> bool {
    store.update(reference, |insight| {
        insight.flagged_important = flag;
    })
}

pub fn delete(store: &InsightStore, reference: &InsightRef) -> bool {
    store.update(reference, |insight| {
        insight.status = InsightStatus::Deleted;
    })
}

pub fn filter(
    store: &InsightStore,
    session: &SessionId,
    kind: Option<InsightKind>,
    keyword: Option<&str>,
) -> Vec<Insight> {
    store
        .visible(session)
        .into_iter()
        .filter(|insight| kind.is_none_or(|expected| insight.kind == expected))
        .filter(|insight| keyword.is_none_or(|needle| insight.text.as_str().contains(needle)))
        .collect()
}

pub fn visible_for_stream(insights: &[Insight]) -> Vec<Insight> {
    insights
        .iter()
        .filter(|insight| is_visible(insight))
        .cloned()
        .collect()
}

pub fn visible_for_minutes(insights: &[Insight]) -> Vec<Insight> {
    insights
        .iter()
        .filter(|insight| is_visible(insight))
        .cloned()
        .collect()
}

pub fn visible_for_relevance(insights: &[Insight]) -> Vec<Insight> {
    insights
        .iter()
        .filter(|insight| is_visible(insight))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::vo::{Confidence, ShortText, UtteranceRef, WallClock};

    fn insight(reference: &str, source: &str, kind: InsightKind, text: &str) -> Insight {
        Insight::new(
            InsightRef::new(reference).expect("insight"),
            kind,
            ShortText::new(text).expect("text"),
            Confidence::new(0.9).expect("conf"),
            vec![UtteranceRef::new(source).expect("ref")],
            WallClock::from_unix_millis(0),
        )
        .expect("insight")
    }

    fn store_with_one() -> (InsightStore, InsightRef, SessionId) {
        let store = InsightStore::default();
        let session = SessionId::new("s1").expect("session");
        store.register_refs(&[UtteranceRef::new("u1").expect("ref")], &session);
        store.insert(insight("i1", "u1", InsightKind::Decision, "确认下周上线"));
        (store, InsightRef::new("i1").expect("insight"), session)
    }

    #[test]
    fn insight_ops_flag_only_changes_flag() {
        let (store, reference, _session) = store_with_one();
        assert!(flag_important(&store, &reference, true));
        let insight = &store.all()[0];
        assert!(insight.flagged_important);
        assert_eq!(insight.mention_count, 1);
        assert_eq!(insight.status, InsightStatus::Auto);
    }

    #[test]
    fn insight_ops_delete_only_changes_status() {
        let (store, reference, session) = store_with_one();
        assert!(delete(&store, &reference));
        assert_eq!(store.all()[0].status, InsightStatus::Deleted);
        assert!(store.visible(&session).is_empty());
    }

    #[test]
    fn insight_ops_filter_is_read_only() {
        let (store, _reference, session) = store_with_one();
        let filtered = filter(&store, &session, Some(InsightKind::Decision), Some("上线"));
        assert_eq!(filtered.len(), 1);
        assert!(filter(&store, &session, Some(InsightKind::Todo), None).is_empty());
        assert_eq!(store.all()[0].mention_count, 1);
    }

    #[test]
    fn deleted_insight_is_hidden_in_three_consumers() {
        let (_store, _reference, _session) = store_with_one();
        let mut insight = insight("i2", "u1", InsightKind::KeyInfo, "回滚方案");
        insight.status = InsightStatus::Deleted;
        let all = vec![insight];
        assert!(visible_for_stream(&all).is_empty());
        assert!(visible_for_minutes(&all).is_empty());
        assert!(visible_for_relevance(&all).is_empty());
    }
}
