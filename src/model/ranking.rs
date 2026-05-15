use std::cmp;

pub struct WeightedText<'a> {
    text: &'a str,
    weight: i32,
}

impl<'a> WeightedText<'a> {
    pub fn new(text: &'a str, weight: i32) -> Self {
        Self { text, weight }
    }
}

pub fn weighted_match_score(query: &str, fields: &[WeightedText<'_>]) -> Option<i32> {
    let query = NormalizedText::new(query);

    if query.compact.is_empty() {
        return None;
    }

    let normalized_fields = fields
        .iter()
        .filter_map(|field| {
            let normalized = NormalizedText::new(field.text);
            (!normalized.compact.is_empty()).then_some((field.weight, normalized))
        })
        .collect::<Vec<_>>();

    let phrase_score = normalized_fields
        .iter()
        .filter_map(|(weight, field)| single_term_score(field, &query).map(|score| score + weight))
        .max();

    let token_scores = query
        .words
        .iter()
        .map(|token| {
            let token = NormalizedText::from_word(token);
            normalized_fields
                .iter()
                .filter_map(|(weight, field)| {
                    single_term_score(field, &token).map(|score| score + weight)
                })
                .max()
        })
        .collect::<Option<Vec<_>>>();

    let token_score = token_scores.map(|scores| {
        let sum = scores.iter().sum::<i32>();
        let coverage_bonus = cmp::min(scores.len() as i32, 4) * 85;
        (sum / scores.len() as i32) + coverage_bonus
    });

    match (phrase_score, token_score) {
        (Some(phrase), Some(tokens)) => {
            Some(cmp::max(phrase, tokens) + cmp::min(phrase, tokens) / 5)
        }
        (Some(phrase), None) => Some(phrase),
        (None, Some(tokens)) => Some(tokens),
        (None, None) => None,
    }
}

pub fn single_term_score(haystack: &NormalizedText, query: &NormalizedText) -> Option<i32> {
    if haystack.compact == query.compact {
        return Some(2_500 - haystack.compact.len() as i32);
    }

    if haystack.compact.starts_with(&query.compact) {
        return Some(2_260 - haystack.compact.len() as i32);
    }

    if let Some(index) = haystack.compact.find(&query.compact) {
        let boundary_bonus = haystack
            .boundaries
            .contains(&index)
            .then_some(180)
            .unwrap_or(0);
        return Some(1_930 + boundary_bonus - (index as i32 * 9) - haystack.compact.len() as i32);
    }

    if !haystack.acronym.is_empty() {
        if haystack.acronym == query.compact {
            return Some(2_180 - haystack.compact.len() as i32);
        }

        if haystack.acronym.starts_with(&query.compact) {
            return Some(2_020 - haystack.compact.len() as i32);
        }
    }

    let word_prefix = haystack
        .words
        .iter()
        .enumerate()
        .find(|(_, word)| word.starts_with(&query.compact))
        .map(|(index, word)| 1_980 - (index as i32 * 55) - word.len() as i32);

    if word_prefix.is_some() {
        return word_prefix;
    }

    if let Some(score) = compact_subsequence_score(haystack, &query.compact) {
        return Some(score);
    }

    typo_tolerant_score(haystack, &query.compact)
}

#[derive(Debug)]
pub struct NormalizedText {
    compact: String,
    words: Vec<String>,
    acronym: String,
    boundaries: Vec<usize>,
}

impl NormalizedText {
    pub fn new(text: &str) -> Self {
        let mut compact = String::new();
        let mut words = Vec::<String>::new();
        let mut current_word = String::new();
        let mut boundaries = Vec::new();
        let mut previous_was_lower = false;

        for character in text.chars() {
            if !character.is_alphanumeric() {
                push_word(&mut words, &mut current_word);
                previous_was_lower = false;
                continue;
            }

            let lower = character.to_lowercase().next().unwrap_or(character);
            let starts_camel_word = previous_was_lower && character.is_uppercase();

            if current_word.is_empty() || starts_camel_word {
                if starts_camel_word {
                    push_word(&mut words, &mut current_word);
                }
                boundaries.push(compact.len());
            }

            compact.push(lower);
            current_word.push(lower);
            previous_was_lower = character.is_lowercase();
        }

        push_word(&mut words, &mut current_word);
        let acronym = words
            .iter()
            .filter_map(|word| word.chars().next())
            .collect();

        Self {
            compact,
            words,
            acronym,
            boundaries,
        }
    }

    pub fn from_word(word: &str) -> Self {
        Self::new(word)
    }
}

fn push_word(words: &mut Vec<String>, current_word: &mut String) {
    if current_word.is_empty() {
        return;
    }

    words.push(std::mem::take(current_word));
}

fn compact_subsequence_score(haystack: &NormalizedText, query: &str) -> Option<i32> {
    let mut query_chars = query.chars();
    let mut current = query_chars.next()?;
    let mut first_match = None;
    let mut previous_index = 0usize;
    let mut matched = 0usize;
    let mut gap_penalty = 0i32;
    let mut consecutive_bonus = 0i32;
    let mut boundary_bonus = 0i32;

    for (index, candidate) in haystack.compact.chars().enumerate() {
        if candidate != current {
            continue;
        }

        if first_match.is_none() {
            first_match = Some(index);
            boundary_bonus += haystack
                .boundaries
                .contains(&index)
                .then_some(90)
                .unwrap_or(0);
        } else {
            let gap = index.saturating_sub(previous_index + 1);
            gap_penalty += gap as i32 * 13;
            if gap == 0 {
                consecutive_bonus += 42;
            }
            boundary_bonus += haystack
                .boundaries
                .contains(&index)
                .then_some(45)
                .unwrap_or(0);
        }

        previous_index = index;
        matched += 1;

        if let Some(next) = query_chars.next() {
            current = next;
        } else {
            let first = first_match.unwrap_or(0);
            let unmatched = haystack.compact.len().saturating_sub(matched);
            return Some(
                1_340 + consecutive_bonus + boundary_bonus
                    - gap_penalty
                    - (first as i32 * 7)
                    - (unmatched as i32 * 3),
            );
        }
    }

    None
}

fn typo_tolerant_score(haystack: &NormalizedText, query: &str) -> Option<i32> {
    if query.len() < 3 {
        return None;
    }

    let allowed_distance = if query.len() <= 5 { 1 } else { 2 };

    haystack
        .words
        .iter()
        .chain(std::iter::once(&haystack.compact))
        .filter_map(|candidate| {
            let distance = bounded_levenshtein(candidate, query, allowed_distance)?;
            Some(1_080 - (distance as i32 * 170) - candidate.len() as i32)
        })
        .max()
}

fn bounded_levenshtein(left: &str, right: &str, max_distance: usize) -> Option<usize> {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();

    if left_chars.len().abs_diff(right_chars.len()) > max_distance {
        return None;
    }

    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left_chars.iter().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];

        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != right_char);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current[right_index + 1] = substitution.min(insertion).min(deletion);
            row_min = row_min.min(current[right_index + 1]);
        }

        if row_min > max_distance {
            return None;
        }

        std::mem::swap(&mut previous, &mut current);
    }

    (previous[right_chars.len()] <= max_distance).then_some(previous[right_chars.len()])
}

#[cfg(test)]
mod tests {
    use super::{NormalizedText, WeightedText, single_term_score, weighted_match_score};

    #[test]
    fn scores_word_prefix_above_subsequence() {
        let target = NormalizedText::new("Visual Studio Code");

        let prefix = single_term_score(&target, &NormalizedText::new("vis")).unwrap();
        let subsequence = single_term_score(&target, &NormalizedText::new("vsc")).unwrap();

        assert!(prefix > subsequence);
    }

    #[test]
    fn supports_acronym_queries() {
        let score = weighted_match_score("vsc", &[WeightedText::new("Visual Studio Code", 760)]);

        assert!(score.is_some());
    }

    #[test]
    fn supports_multi_token_queries() {
        let score = weighted_match_score(
            "studio code",
            &[WeightedText::new("Visual Studio Code", 760)],
        );

        assert!(score.is_some());
    }

    #[test]
    fn tolerates_small_typos() {
        let score = weighted_match_score("vesktop", &[WeightedText::new("Vesktop", 760)]);
        let typo_score = weighted_match_score("vesktpo", &[WeightedText::new("Vesktop", 760)]);

        assert!(score.unwrap() > typo_score.unwrap());
    }
}
