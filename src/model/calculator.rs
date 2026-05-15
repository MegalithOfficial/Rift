use crate::model::{ResultAction, SearchResult};

pub fn calculator_result(query: &str) -> Option<SearchResult> {
    if !looks_like_expression(query) {
        return None;
    }

    let expression = normalize_expression(query)?;
    let value = meval::eval_str(&expression).ok()?;
    if !value.is_finite() {
        return None;
    }

    let rendered = format_number(value);

    Some(SearchResult::new(
        rendered.clone(),
        format!("{query} = {rendered}"),
        String::new(),
        None,
        "accessories-calculator-symbolic",
        format!("calc:{query}"),
        ResultAction::CopyText(rendered),
    ))
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
    use crate::model::{ResultAction, SearchResult};

    use super::calculator_result;

    fn copied_value(result: &SearchResult) -> Option<&str> {
        match result.action() {
            ResultAction::CopyText(value) => Some(value.as_str()),
            _ => None,
        }
    }

    #[test]
    fn detects_spaced_calculator_expression() {
        let result = calculator_result("2 + 2").unwrap();

        assert_eq!(result.title(), "4");
        assert_eq!(copied_value(&result), Some("4"));
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
