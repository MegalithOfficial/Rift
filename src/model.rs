use std::{
    cell::RefCell,
    cmp,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use gio::prelude::*;

const MAX_RESULTS: usize = 8;

#[derive(Clone)]
pub enum ResultAction {
    LaunchApp(gio::AppInfo),
    RunShell(String),
    CopyText(String),
}

#[derive(Clone)]
pub struct SearchResult {
    title: String,
    subtitle: String,
    executable: String,
    icon: Option<gio::Icon>,
    fallback_icon_name: &'static str,
    action: ResultAction,
    usage_key: String,
}

impl SearchResult {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn subtitle(&self) -> &str {
        &self.subtitle
    }

    pub fn executable(&self) -> &str {
        &self.executable
    }

    pub fn icon(&self) -> Option<&gio::Icon> {
        self.icon.as_ref()
    }

    pub fn fallback_icon_name(&self) -> &'static str {
        self.fallback_icon_name
    }

    pub fn action(&self) -> &ResultAction {
        &self.action
    }

    pub fn usage_key(&self) -> &str {
        &self.usage_key
    }
}

#[derive(Clone)]
struct DesktopEntry {
    app_info: gio::AppInfo,
    app_id: String,
    display_name: String,
    description: String,
    executable: String,
    icon: Option<gio::Icon>,
}

impl DesktopEntry {
    fn load() -> Vec<Self> {
        let mut apps = gio::AppInfo::all()
            .into_iter()
            .filter(|app| app.should_show())
            .map(|app| {
                let display_name = app.display_name().to_string();
                let description = app
                    .description()
                    .map(|text| text.to_string())
                    .unwrap_or_else(|| app.name().to_string());
                let executable = app.executable().to_string_lossy().into_owned();
                let icon = app.icon();
                let app_id = app
                    .id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| executable.clone());

                Self {
                    app_info: app,
                    app_id,
                    display_name,
                    description,
                    executable,
                    icon,
                }
            })
            .collect::<Vec<_>>();

        apps.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        apps
    }

    fn into_result(self) -> SearchResult {
        SearchResult {
            title: self.display_name,
            subtitle: self.description,
            executable: self.executable,
            icon: self.icon,
            fallback_icon_name: "application-x-executable-symbolic",
            usage_key: format!("app:{}", self.app_id),
            action: ResultAction::LaunchApp(self.app_info),
        }
    }

    fn score(&self, query: &str) -> Option<i32> {
        if query.is_empty() {
            return Some(0);
        }

        weighted_match_score(
            query,
            &[
                WeightedText::new(&self.display_name, 760),
                WeightedText::new(&self.app_id, 360),
                WeightedText::new(&self.executable, 260),
                WeightedText::new(&self.description, 80),
            ],
        )
    }
}

pub struct AppIndex {
    entries: Vec<DesktopEntry>,
    history: RefCell<HashMap<String, u32>>,
    history_path: PathBuf,
}

impl AppIndex {
    pub fn load() -> Self {
        let history_path = history_path();
        let history = read_history(&history_path);

        Self {
            entries: DesktopEntry::load(),
            history: RefCell::new(history),
            history_path,
        }
    }

    pub fn query(&self, term: &str) -> Vec<SearchResult> {
        let trimmed = term.trim();

        if trimmed.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        if let Some(result) = shell_result(trimmed) {
            results.push((1_400, result));
        }

        if let Some(result) = calculator_result(trimmed) {
            results.push((10_000, result));
        }

        let history = self.history.borrow();
        let remaining_slots = MAX_RESULTS.saturating_sub(results.len());
        if remaining_slots > 0 {
            let mut app_matches = self
                .entries
                .iter()
                .filter_map(|entry| {
                    entry.score(trimmed).map(|score| {
                        let usage_bonus =
                            history_bonus(history.get(&format!("app:{}", entry.app_id)));
                        (
                            score + usage_bonus - (entry.display_name.len() as i32 / 3),
                            entry.clone(),
                        )
                    })
                })
                .collect::<Vec<_>>();

            app_matches.sort_by(|(left_score, left), (right_score, right)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left.display_name.cmp(&right.display_name))
            });

            results.extend(
                app_matches
                    .into_iter()
                    .map(|(score, entry)| (score, entry.into_result()))
                    .take(remaining_slots),
            );
        }

        results.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.title.cmp(&right.title))
        });

        results
            .into_iter()
            .map(|(_, result)| result)
            .take(MAX_RESULTS)
            .collect()
    }

    pub fn record_usage(&self, result: &SearchResult) {
        let mut history = self.history.borrow_mut();
        let entry = history.entry(result.usage_key().to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        let snapshot = history.clone();
        drop(history);
        let _ = write_history(&self.history_path, &snapshot);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

fn shell_result(query: &str) -> Option<SearchResult> {
    let command = query.strip_prefix('>')?.trim();

    if command.is_empty() {
        return None;
    }

    Some(SearchResult {
        title: command.to_string(),
        subtitle: "Run in terminal".to_string(),
        executable: command.to_string(),
        icon: None,
        fallback_icon_name: "utilities-terminal-symbolic",
        usage_key: format!("shell:{command}"),
        action: ResultAction::RunShell(command.to_string()),
    })
}

fn calculator_result(query: &str) -> Option<SearchResult> {
    if !looks_like_expression(query) {
        return None;
    }

    let expression = normalize_expression(query)?;
    let value = meval::eval_str(&expression).ok()?;
    if !value.is_finite() {
        return None;
    }

    let rendered = format_number(value);

    Some(SearchResult {
        title: rendered.clone(),
        subtitle: format!("{query} = {rendered}"),
        executable: String::new(),
        icon: None,
        fallback_icon_name: "accessories-calculator-symbolic",
        usage_key: format!("calc:{query}"),
        action: ResultAction::CopyText(rendered),
    })
}

fn normalize_expression(query: &str) -> Option<String> {
    let lower = query.trim().to_ascii_lowercase();
    let normalized = lower
        .replace('×', "*")
        .replace('÷', "/")
        .replace('−', "-")
        .replace(',', ".");

    if let Some((percent, base)) = normalized.split_once(" of ") {
        let percent = percent.trim().strip_suffix('%')?.trim();
        let base = normalize_expression(base)?;
        return Some(format!("(({percent})/100)*({base})"));
    }

    rewrite_percentages(&normalized)
}

struct WeightedText<'a> {
    text: &'a str,
    weight: i32,
}

impl<'a> WeightedText<'a> {
    fn new(text: &'a str, weight: i32) -> Self {
        Self { text, weight }
    }
}

fn weighted_match_score(query: &str, fields: &[WeightedText<'_>]) -> Option<i32> {
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

fn single_term_score(haystack: &NormalizedText, query: &NormalizedText) -> Option<i32> {
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
struct NormalizedText {
    compact: String,
    words: Vec<String>,
    acronym: String,
    boundaries: Vec<usize>,
}

impl NormalizedText {
    fn new(text: &str) -> Self {
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

    fn from_word(word: &str) -> Self {
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

fn history_bonus(count: Option<&u32>) -> i32 {
    count.copied().unwrap_or(0).min(8).saturating_mul(45) as i32
}

fn history_path() -> PathBuf {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("rift").join("history.tsv")
}

fn read_history(path: &Path) -> HashMap<String, u32> {
    let mut history = HashMap::new();
    let Ok(contents) = fs::read_to_string(path) else {
        return history;
    };

    for line in contents.lines() {
        let Some((key, count)) = line.split_once('\t') else {
            continue;
        };

        if let Ok(count) = count.parse::<u32>() {
            history.insert(key.to_string(), count);
        }
    }

    history
}

fn write_history(path: &Path, history: &HashMap<String, u32>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut lines = history
        .iter()
        .map(|(key, count)| format!("{key}\t{count}"))
        .collect::<Vec<_>>();
    lines.sort();
    fs::write(path, lines.join("\n"))
}

fn looks_like_expression(query: &str) -> bool {
    let mut has_digit = false;
    let mut has_operator_or_function = false;
    let mut identifiers = Vec::new();
    let mut current_identifier = String::new();

    for character in query.trim().chars() {
        if character.is_ascii_digit() {
            has_digit = true;
            push_identifier(&mut identifiers, &mut current_identifier);
            continue;
        }

        if matches!(
            character,
            ' ' | '\t'
                | '.'
                | ','
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '^'
                | '('
                | ')'
                | '×'
                | '÷'
                | '−'
        ) {
            push_identifier(&mut identifiers, &mut current_identifier);
            if !character.is_whitespace() && character != '.' && character != ',' {
                has_operator_or_function = true;
            }
            continue;
        }

        if character.is_ascii_alphabetic() || character == '_' {
            has_operator_or_function = true;
            current_identifier.push(character.to_ascii_lowercase());
            continue;
        }

        return false;
    }

    push_identifier(&mut identifiers, &mut current_identifier);
    let has_calculator_identifier = identifiers
        .iter()
        .any(|identifier| is_calculator_identifier(identifier));

    (has_digit || has_calculator_identifier) && has_operator_or_function
}

fn push_identifier(identifiers: &mut Vec<String>, current_identifier: &mut String) {
    if current_identifier.is_empty() {
        return;
    }

    identifiers.push(std::mem::take(current_identifier));
}

fn is_calculator_identifier(identifier: &str) -> bool {
    matches!(
        identifier,
        "pi" | "e"
            | "sqrt"
            | "abs"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "sinh"
            | "cosh"
            | "tanh"
            | "asinh"
            | "acosh"
            | "atanh"
            | "exp"
            | "ln"
            | "floor"
            | "ceil"
            | "round"
            | "signum"
            | "min"
            | "max"
    )
}

fn rewrite_percentages(expression: &str) -> Option<String> {
    let chars = expression.chars().collect::<Vec<_>>();
    let mut rewritten = String::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index].is_whitespace() {
            index += 1;
            continue;
        }

        if is_number_start(&chars, index) {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }

            let number = chars[start..index].iter().collect::<String>();
            if index < chars.len() && chars[index] == '%' {
                rewritten.push_str("((");
                rewritten.push_str(&number);
                rewritten.push_str(")/100)");
                index += 1;
            } else {
                rewritten.push_str(&number);
            }
            continue;
        }

        rewritten.push(chars[index]);
        index += 1;
    }

    (!rewritten.is_empty()).then_some(rewritten)
}

fn is_number_start(chars: &[char], index: usize) -> bool {
    if chars[index].is_ascii_digit() {
        return true;
    }

    chars[index] == '.'
        && chars
            .get(index + 1)
            .copied()
            .is_some_and(|next| next.is_ascii_digit())
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        let rendered = format!("{value:.10}");
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NormalizedText, ResultAction, WeightedText, calculator_result, single_term_score,
        weighted_match_score,
    };

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

    #[test]
    fn detects_spaced_calculator_expression() {
        let result = calculator_result("2 + 2").unwrap();

        assert_eq!(result.title(), "4");
        assert!(matches!(result.action(), ResultAction::CopyText(value) if value == "4"));
    }

    #[test]
    fn rejects_non_expression_queries() {
        assert!(calculator_result("java 21").is_none());
    }

    #[test]
    fn supports_functions_and_constants() {
        let result = calculator_result("round(sin(pi))").unwrap();

        assert_eq!(result.title(), "0");
    }

    #[test]
    fn supports_unicode_operators() {
        let result = calculator_result("6 × 7").unwrap();

        assert_eq!(result.title(), "42");
    }

    #[test]
    fn supports_percentages() {
        let result = calculator_result("20% of 150").unwrap();

        assert_eq!(result.title(), "30");
    }

    #[test]
    fn supports_inline_percentages() {
        let result = calculator_result("100 + 10%").unwrap();

        assert_eq!(result.title(), "100.1");
    }

    #[test]
    fn supports_decimal_commas() {
        let result = calculator_result("1,5 + 2").unwrap();

        assert_eq!(result.title(), "3.5");
    }
}
