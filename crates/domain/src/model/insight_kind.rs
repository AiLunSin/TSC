//! 四类封闭、兜底与可观测结果。

use crate::model::enums::InsightKind;

/// 分类结果；`fallback == true` 表示走了未知类兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindClassification {
    pub kind: InsightKind,
    pub fallback: bool,
}

/// 全函数：任意输入都返回四类之一，未知值兜底为 `KeyInfo`。
pub fn classify_kind(raw: Option<&str>) -> KindClassification {
    match raw {
        Some("key_info") => KindClassification {
            kind: InsightKind::KeyInfo,
            fallback: false,
        },
        Some("decision") => KindClassification {
            kind: InsightKind::Decision,
            fallback: false,
        },
        Some("todo") => KindClassification {
            kind: InsightKind::Todo,
            fallback: false,
        },
        Some("question") => KindClassification {
            kind: InsightKind::Question,
            fallback: false,
        },
        _ => KindClassification {
            kind: InsightKind::KeyInfo,
            fallback: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_classify_is_total() {
        assert_eq!(classify_kind(Some("key_info")).kind, InsightKind::KeyInfo);
        assert_eq!(classify_kind(Some("decision")).kind, InsightKind::Decision);
        assert_eq!(classify_kind(Some("todo")).kind, InsightKind::Todo);
        assert_eq!(classify_kind(Some("question")).kind, InsightKind::Question);
        assert!(classify_kind(Some("note")).fallback);
        assert!(classify_kind(None).fallback);
        assert!(classify_kind(Some("TODO")).fallback);
        assert!(classify_kind(Some("")).fallback);
    }

    #[test]
    fn kind_fallback_table_matches_spec() {
        let direct = [
            ("key_info", InsightKind::KeyInfo),
            ("decision", InsightKind::Decision),
            ("todo", InsightKind::Todo),
            ("question", InsightKind::Question),
        ];
        for (raw, kind) in direct {
            assert_eq!(
                classify_kind(Some(raw)),
                KindClassification {
                    kind,
                    fallback: false
                }
            );
        }
        for raw in [None, Some("note"), Some("unknown")] {
            assert_eq!(
                classify_kind(raw),
                KindClassification {
                    kind: InsightKind::KeyInfo,
                    fallback: true
                }
            );
        }
    }
}
