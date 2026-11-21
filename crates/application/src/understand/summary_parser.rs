//! C-01 主题解析器：从 `Summary.text` 确定性解析 topics/entities。

use meeting_copilot_domain::merge::normalize_text;
use meeting_copilot_domain::model::insight::{Entity, Topic};
use meeting_copilot_domain::model::rolling_summary::SummaryParse;

use super::ports::LexiconView;

const SENTENCE_BREAKS: &[char] = &['。', '！', '？', '；', '!', '?', ';', '\n'];
const SECONDARY_BREAKS: &[char] = &[',', '，', '、', '.', '。', ' '];
const MAX_SENTENCE_CHARS: usize = 80;
const MAX_TOPICS: usize = 5;
const MAX_ENTITIES: usize = 10;

fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if SENTENCE_BREAKS.contains(&ch) {
            let trimmed = current.trim().to_owned();
            if !trimmed.is_empty() {
                out.push(trimmed);
            }
            current.clear();
        }
    }
    let tail = current.trim().to_owned();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

fn wrap_long(sentence: &str) -> Vec<String> {
    let chars: Vec<char> = sentence.chars().collect();
    if chars.len() <= MAX_SENTENCE_CHARS {
        return vec![sentence.to_owned()];
    }
    let mut out = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + MAX_SENTENCE_CHARS).min(chars.len());
        let mut cut = end;
        if end < chars.len() {
            for index in (start + 1..=end).rev() {
                if SECONDARY_BREAKS.contains(&chars[index - 1]) {
                    cut = index;
                    break;
                }
            }
        }
        out.push(chars[start..cut].iter().collect());
        start = cut;
    }
    out
}

fn ascii_tokens(sentence: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in sentence.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            current.push(ch);
        } else {
            if current.len() >= 2 {
                out.push(current.clone());
            }
            current.clear();
        }
    }
    if current.len() >= 2 {
        out.push(current);
    }
    out
}

fn bigram_fallback(sentence: &str) -> Option<String> {
    let normalized = normalize_text(sentence);
    let chars: Vec<char> = normalized.chars().collect();
    if chars.len() < 2 {
        return None;
    }
    let mut counts: Vec<(String, u32, usize)> = Vec::new();
    for width in 2..=12.min(chars.len()) {
        for start in 0..=chars.len() - width {
            let gram: String = chars[start..start + width].iter().collect();
            match counts.iter_mut().find(|(label, ..)| *label == gram) {
                Some((_, count, _)) => *count += 1,
                None => counts.push((gram, 1, start)),
            }
        }
    }
    counts.retain(|(_, count, _)| *count >= 2);
    counts.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(left.2.cmp(&right.2))
            .then(left.0.as_bytes().cmp(right.0.as_bytes()))
    });
    counts.first().map(|(label, ..)| label.clone())
}

/// 只接收正文与术语快照；`covered`/`version` 由调用方负责。
pub fn parse_summary(text: &str, lexicon: &LexiconView) -> SummaryParse {
    let mut candidates: Vec<(String, u32, usize)> = Vec::new();
    let mut position = 0usize;
    let mut entity_counts: Vec<(String, u32)> = Vec::new();

    let mut push_candidate = |label: String, weight: u32, first: usize| match candidates
        .iter_mut()
        .find(|(existing, ..)| *existing == label)
    {
        Some((_, count, _)) => *count += weight,
        None => candidates.push((label, weight, first)),
    };

    for sentence in sentences(text) {
        for piece in wrap_long(&sentence) {
            let mut hits = Vec::new();
            for name in lexicon.names() {
                let count = u32::try_from(piece.matches(name).count()).unwrap_or(u32::MAX);
                if count > 0 {
                    hits.push((name.clone(), count));
                }
            }
            if hits.is_empty() {
                if let Some(gram) = bigram_fallback(&piece) {
                    push_candidate(gram, 1, position);
                }
            } else {
                for (name, count) in hits {
                    push_candidate(name, count, position);
                }
            }

            for token in ascii_tokens(&piece) {
                match entity_counts
                    .iter_mut()
                    .find(|(existing, ..)| *existing == token)
                {
                    Some((_, count)) => *count += 1,
                    None => entity_counts.push((token, 1)),
                }
            }
            for name in lexicon.names() {
                if piece.contains(name) {
                    match entity_counts
                        .iter_mut()
                        .find(|(existing, ..)| *existing == *name)
                    {
                        Some((_, count)) => *count += 1,
                        None => entity_counts.push((name.clone(), 1)),
                    }
                }
            }
            position += piece.chars().count();
        }
    }

    let total: u32 = candidates.iter().map(|(_, count, _)| *count).sum();
    candidates.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(left.2.cmp(&right.2))
            .then(left.0.as_bytes().cmp(right.0.as_bytes()))
    });
    let topics: Vec<Topic> = candidates
        .into_iter()
        .take(MAX_TOPICS)
        .map(|(label, count, _)| Topic {
            label,
            weight: if total == 0 {
                0.0
            } else {
                f64::from(count) / f64::from(total)
            },
        })
        .collect();

    let mut entities: Vec<Entity> = entity_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(name, _)| Entity {
            normalized: normalize_text(&name),
            name,
        })
        .collect();
    entities.dedup_by(|left, right| left.normalized == right.normalized);
    entities.truncate(MAX_ENTITIES);

    SummaryParse { topics, entities }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon(names: &[&str]) -> LexiconView {
        LexiconView::from_names(names.iter().map(|name| (*name).to_owned()).collect())
    }

    #[test]
    fn parse_summary_splits_sentences_deterministically() {
        let text = "灰度上线。回滚方案！监控告警？";
        let first = parse_summary(text, &LexiconView::default());
        for _ in 0..100 {
            assert_eq!(first, parse_summary(text, &LexiconView::default()));
        }
    }

    #[test]
    fn summary_parser_prefers_lexicon_hits() {
        let parsed = parse_summary(
            "灰度发布。灰度回滚。灰度监控。",
            &lexicon(&["灰度发布", "灰度回滚", "监控告警"]),
        );
        assert!(!parsed.topics.is_empty());
        assert_eq!(parsed.topics[0].label, "灰度发布");
    }

    #[test]
    fn summary_parser_falls_back_to_frequent_bigrams() {
        let parsed = parse_summary("回滚回滚。回滚回滚。", &LexiconView::default());
        assert!(!parsed.topics.is_empty());
        assert!(parsed.topics[0].label.contains("回滚"));
    }

    #[test]
    fn summary_topics_are_truncated_to_five() {
        let parsed = parse_summary(
            "a1 a2 a3 a4 a5 a6 a7",
            &lexicon(&["a1", "a2", "a3", "a4", "a5", "a6", "a7"]),
        );
        assert!(parsed.topics.len() <= 5);
    }

    #[test]
    fn summary_entities_are_normalized_and_deduped() {
        let parsed = parse_summary(
            "张三负责回滚。张三确认监控。ABC-123 演练。ABC-123 复盘。",
            &lexicon(&["张三"]),
        );
        assert!(parsed.entities.iter().any(|entity| entity.name == "张三"));
        assert!(parsed
            .entities
            .iter()
            .any(|entity| entity.name == "ABC-123"));
        assert!(parsed.entities.len() <= 10);
    }

    #[test]
    fn summary_without_topics_returns_empty() {
        assert!(parse_summary("", &LexiconView::default()).topics.is_empty());
        assert!(parse_summary("…", &LexiconView::default())
            .topics
            .is_empty());
    }
}
