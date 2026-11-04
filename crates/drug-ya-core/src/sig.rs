//! Compose the complete HOSxP directions-for-use text for display.
//!
//! HOSxP spreads one prescription instruction across `drugusage.name1..3`
//! and `sp_use.name1..3`. The old connector picked only the longest single
//! value, which dropped details such as "หลังอาหาร" or a special-use note.
//! This module joins every non-empty part, removes exact duplicates, and
//! drops a short part when a longer part already contains it.

/// Join every non-empty sig part into one display string.
///
/// Parts are trimmed and internal whitespace is collapsed. A short code such
/// as `1x3` is dropped when the same code is already contained in a longer
/// note such as `1x3 หลังอาหาร`. The result is joined with ` · `.
pub fn compose_sig(parts: &[String]) -> String {
    let mut cleaned: Vec<String> = Vec::new();
    for part in parts {
        let text = part.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            continue;
        }
        let key = text.to_lowercase();
        if cleaned
            .iter()
            .any(|existing| existing.to_lowercase() == key)
        {
            continue;
        }
        cleaned.push(text);
    }

    let mut kept: Vec<String> = Vec::new();
    for (index, part) in cleaned.iter().enumerate() {
        let lower = part.to_lowercase();
        let part_len = part.chars().count();
        let contained = cleaned.iter().enumerate().any(|(other_index, other)| {
            if other_index == index {
                return false;
            }
            let other_lower = other.to_lowercase();
            other_lower.contains(&lower)
                && (other.chars().count() > part_len || other_index < index)
        });
        if !contained {
            kept.push(part.clone());
        }
    }

    kept.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::compose_sig;

    fn parts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn joins_all_non_empty_parts_in_source_order() {
        assert_eq!(
            compose_sig(&parts(&["1x3", "หลังอาหาร", "รับประทานครั้งละ 1 เม็ด"])),
            "1x3 · หลังอาหาร · รับประทานครั้งละ 1 เม็ด"
        );
    }

    #[test]
    fn trims_empty_values_and_collapses_whitespace() {
        assert_eq!(
            compose_sig(&parts(&["  1x3  ", "", "   ", "หลัง   อาหาร"])),
            "1x3 · หลัง อาหาร"
        );
    }

    #[test]
    fn removes_exact_duplicates_case_insensitively() {
        assert_eq!(
            compose_sig(&parts(&["Take with food", "take with food", "1x3"])),
            "Take with food · 1x3"
        );
    }

    #[test]
    fn drops_short_parts_contained_in_a_longer_part() {
        assert_eq!(
            compose_sig(&parts(&["1x3", "หลังอาหาร", "1x3 หลังอาหาร เช้า กลางวัน เย็น"])),
            "1x3 หลังอาหาร เช้า กลางวัน เย็น"
        );
    }

    #[test]
    fn keeps_distinct_short_codes_and_notes() {
        assert_eq!(
            compose_sig(&parts(&["1x3", "รับประทานครั้งละ 1 เม็ด"])),
            "1x3 · รับประทานครั้งละ 1 เม็ด"
        );
    }

    #[test]
    fn all_empty_parts_become_an_empty_string() {
        assert_eq!(compose_sig(&parts(&["", "   ", "\t"])), "");
        assert_eq!(compose_sig(&[]), "");
    }
}
