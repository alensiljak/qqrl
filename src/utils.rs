use regex::Regex;
use rust_decimal::Decimal;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UtilsError {
    #[error("Invalid amount filter format: {0}")]
    InvalidAmountFilter(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountParams {
    pub account_regexes: Vec<String>,
    pub excluded_account_regexes: Vec<String>,
    pub where_clauses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmountFilter {
    pub operator: String,
    pub value: Decimal,
    pub currency: Option<String>,
}

/// Convert account CLI patterns into regex patterns used by query builders.
pub fn parse_account_pattern(pattern: &str) -> String {
    if pattern.starts_with('^') && pattern.ends_with('$') && pattern.len() >= 2 {
        let exact = &pattern[1..pattern.len() - 1];
        return format!("^{}$", regex::escape(exact));
    }

    if let Some(starts_with) = pattern.strip_prefix('^') {
        return format!("^{}", regex::escape(starts_with));
    }

    if let Some(ends_with) = pattern.strip_suffix('$') {
        return format!("{}$", regex::escape(ends_with));
    }

    pattern.to_string()
}

/// Parse account regex args into include patterns, excluded patterns, and payee filters.
pub fn parse_account_params(account_regex: &[String]) -> AccountParams {
    let mut account_regexes = Vec::new();
    let mut excluded_account_regexes = Vec::new();
    let mut where_clauses = Vec::new();

    let mut i = 0;
    while i < account_regex.len() {
        let current = &account_regex[i];

        if current == "not" {
            i += 1;
            while i < account_regex.len() {
                let next = &account_regex[i];
                if next.starts_with('@') || next == "not" {
                    i = i.saturating_sub(1);
                    break;
                }
                excluded_account_regexes.push(next.clone());
                i += 1;
            }
        } else if let Some(payee) = current.strip_prefix('@') {
            where_clauses.push(format!("description ~ '{payee}'"));
        } else {
            account_regexes.push(current.clone());
        }

        i += 1;
    }

    AccountParams {
        account_regexes,
        excluded_account_regexes,
        where_clauses,
    }
}

/// Parse amount filter like `>100EUR` into operator, decimal value, and optional currency.
pub fn parse_amount_filter(amount_str: &str) -> Result<AmountFilter, UtilsError> {
    let regex =
        Regex::new(r"(?i)^([><]=?|=)?(-?\d+\.?\d*)([A-Z]{3})?").expect("amount regex must compile");

    let captures = regex
        .captures(amount_str)
        .ok_or_else(|| UtilsError::InvalidAmountFilter(amount_str.to_string()))?;

    let operator = captures
        .get(1)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "=".to_string());
    let value_str = captures
        .get(2)
        .map(|m| m.as_str())
        .ok_or_else(|| UtilsError::InvalidAmountFilter(amount_str.to_string()))?;
    let value = value_str
        .parse::<Decimal>()
        .map_err(|_| UtilsError::InvalidAmountFilter(amount_str.to_string()))?;
    let currency = captures.get(3).map(|m| m.as_str().to_uppercase());

    Ok(AmountFilter {
        operator,
        value,
        currency,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_account_params, parse_account_pattern, parse_amount_filter};

    #[test]
    fn parse_account_pattern_modes() {
        assert_eq!(parse_account_pattern("^Assets$"), "^Assets$");
        assert_eq!(parse_account_pattern("^Assets:Bank"), "^Assets:Bank");
        assert_eq!(parse_account_pattern("Bank$"), "Bank$");
        assert_eq!(parse_account_pattern("Assets:.*"), "Assets:.*");
    }

    #[test]
    fn parse_account_params_not_and_payee() {
        let input = vec![
            "Assets".to_string(),
            "not".to_string(),
            "Liabilities".to_string(),
            "@amazon".to_string(),
            "Income".to_string(),
        ];
        let parsed = parse_account_params(&input);

        assert_eq!(parsed.account_regexes, vec!["Assets", "Income"]);
        assert_eq!(parsed.excluded_account_regexes, vec!["Liabilities"]);
        assert_eq!(parsed.where_clauses, vec!["description ~ 'amazon'"]);
    }

    #[test]
    fn parse_amount_filter_variants() {
        let filter = parse_amount_filter(">=100.50eur").unwrap();
        assert_eq!(filter.operator, ">=");
        assert_eq!(filter.value.to_string(), "100.50");
        assert_eq!(filter.currency, Some("EUR".to_string()));

        let filter = parse_amount_filter("42").unwrap();
        assert_eq!(filter.operator, "=");
        assert_eq!(filter.value.to_string(), "42");
        assert_eq!(filter.currency, None);
    }
}
