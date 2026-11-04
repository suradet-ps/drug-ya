//! Patient search input classification.
//!
//! The pharmacy search box accepts:
//! - HN: 5-10 digits, or an explicit `HN` prefix
//! - CID: exactly 13 digits
//! - queue: `Q` / `QH` / `คิว` prefix, or the explicit queue mode
//! - name: anything else

use serde::{Deserialize, Serialize};

/// The search modes offered by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SearchKind {
    /// Infer from the input shape.
    #[default]
    Auto,
    /// Hospital number.
    Hn,
    /// Thai national ID.
    Cid,
    /// Patient name.
    Name,
    /// Today's queue number.
    Queue,
}

/// A parsed search term and its resolved kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSearch {
    /// Resolved search kind (never `Auto`).
    pub kind: SearchKind,
    /// The term to bind into the query, with mode prefixes removed.
    pub term: String,
}

/// Parse and classify one search input.
pub fn parse_search(input: &str, requested: SearchKind) -> ParsedSearch {
    let trimmed = input.trim();
    if requested != SearchKind::Auto {
        return ParsedSearch {
            kind: requested,
            term: strip_prefix(trimmed, requested),
        };
    }

    let lower = trimmed.to_lowercase();
    if let Some(rest) = strip_queue_prefix(&lower) {
        return ParsedSearch {
            kind: SearchKind::Queue,
            term: rest,
        };
    }
    if let Some(rest) = lower.strip_prefix("hn") {
        let rest = rest.trim_start_matches([' ', ':', '-', '/']);
        if !rest.is_empty() {
            return ParsedSearch {
                kind: SearchKind::Hn,
                term: rest.to_string(),
            };
        }
    }
    if trimmed.len() == 13 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return ParsedSearch {
            kind: SearchKind::Cid,
            term: trimmed.to_string(),
        };
    }
    if (5..=10).contains(&trimmed.len()) && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return ParsedSearch {
            kind: SearchKind::Hn,
            term: trimmed.to_string(),
        };
    }
    if (1..=4).contains(&trimmed.len()) && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return ParsedSearch {
            kind: SearchKind::Queue,
            term: trimmed.to_string(),
        };
    }
    ParsedSearch {
        kind: SearchKind::Name,
        term: trimmed.to_string(),
    }
}

fn strip_queue_prefix(lower: &str) -> Option<String> {
    for prefix in ["qh", "q", "คิว"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let rest = rest.trim_start_matches([' ', ':', '-', '/', '.']);
            if !rest.is_empty() && rest.chars().any(|c| c.is_ascii_alphanumeric()) {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn strip_prefix(input: &str, kind: SearchKind) -> String {
    match kind {
        SearchKind::Queue => {
            strip_queue_prefix(&input.to_lowercase()).unwrap_or_else(|| input.to_string())
        }
        SearchKind::Hn => {
            let lower = input.to_lowercase();
            if let Some(rest) = lower.strip_prefix("hn") {
                rest.trim_start_matches([' ', ':', '-', '/']).to_string()
            } else {
                input.to_string()
            }
        }
        SearchKind::Cid | SearchKind::Name | SearchKind::Auto => input.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thirteen_digits_is_cid() {
        let p = parse_search("1101701234567", SearchKind::Auto);
        assert_eq!(p.kind, SearchKind::Cid);
        assert_eq!(p.term, "1101701234567");
    }

    #[test]
    fn five_to_ten_digits_is_hn() {
        assert_eq!(parse_search("12345", SearchKind::Auto).kind, SearchKind::Hn);
        assert_eq!(
            parse_search("1234567890", SearchKind::Auto).kind,
            SearchKind::Hn
        );
    }

    #[test]
    fn short_digits_are_a_queue_in_auto_mode() {
        let p = parse_search("123", SearchKind::Auto);
        assert_eq!(p.kind, SearchKind::Queue);
        assert_eq!(p.term, "123");
        let explicit = parse_search("123", SearchKind::Hn);
        assert_eq!(explicit.kind, SearchKind::Hn);
        assert_eq!(explicit.term, "123");
    }

    #[test]
    fn queue_prefixes_are_recognized() {
        for input in ["Q12", "q 12", "QH12", "คิว 12"] {
            let p = parse_search(input, SearchKind::Auto);
            assert_eq!(p.kind, SearchKind::Queue, "{input}");
            assert_eq!(p.term, "12", "{input}");
        }
    }

    #[test]
    fn hn_prefix_is_recognized() {
        let p = parse_search("HN 0012345", SearchKind::Auto);
        assert_eq!(p.kind, SearchKind::Hn);
        assert_eq!(p.term, "0012345");
    }

    #[test]
    fn names_stay_names() {
        assert_eq!(
            parse_search("สมชาย ใจดี", SearchKind::Auto).kind,
            SearchKind::Name
        );
    }
}
