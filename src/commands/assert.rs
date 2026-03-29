use comfy_table::{presets, Cell, CellAlignment, Table};
use rust_decimal::Decimal;
use serde_json::Value;

use crate::{
    cli::CommonOptions,
    config::Config,
    date_parser::{parse_date, parse_date_range},
    runner::run_bql_query,
    utils::{parse_account_params, parse_account_pattern, parse_amount_filter},
};

#[derive(Debug, Clone)]
struct AssertRow {
    date: String,
    account: String,
    balance_number: Decimal,
    balance_currency: String,
}

/// Balance assertions
///
/// Display balance assertions from the `#balances` system table.
///
/// Usage:
///   qqrl assert [PATTERN] [OPTIONS]
///   qqrl a Assets
///   qqrl assert --begin 2025-11-01
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(opts.ledger.clone())?;
    let query = build_query(&opts);
    println!("\nYour BQL query is:\n{query}\n");
    let rows = run_bql_query(&config, &query)?;
    let mut assert_rows = parse_rows(&rows)?;

    // Client-side currency filter
    let currencies: Vec<String> = opts
        .currency
        .iter()
        .flat_map(|c| c.split(',').map(|s| s.trim().to_uppercase()))
        .filter(|s| !s.is_empty())
        .collect();
    if !currencies.is_empty() {
        assert_rows.retain(|r| currencies.contains(&r.balance_currency));
    }

    print_table(&assert_rows);
    Ok(())
}

// ---------------------------------------------------------------------------
// Query builder
// ---------------------------------------------------------------------------

fn build_query(opts: &CommonOptions) -> String {
    let mut where_clauses: Vec<String> = Vec::new();

    // Account patterns (include, exclude, payee)
    let params = parse_account_params(&opts.account);
    for clause in &params.where_clauses {
        where_clauses.push(clause.clone());
    }
    for pattern in &params.account_regexes {
        let regex = parse_account_pattern(pattern);
        where_clauses.push(format!("account ~ '{regex}'"));
    }
    for pattern in &params.excluded_account_regexes {
        let regex = parse_account_pattern(pattern);
        where_clauses.push(format!("NOT (account ~ '{regex}')"));
    }

    // Date filters
    if let Some(begin) = &opts.begin {
        if let Ok(date) = parse_date(begin) {
            where_clauses.push(format!("date >= date(\"{date}\")"));
        }
    }
    if let Some(end) = &opts.end {
        if let Ok(date) = parse_date(end) {
            where_clauses.push(format!("date < date(\"{date}\")"));
        }
    }
    if let Some(range) = &opts.date_range {
        if let Ok((begin, end)) = parse_date_range(range) {
            if let Some(b) = begin {
                where_clauses.push(format!("date >= date(\"{b}\")"));
            }
            if let Some(e) = end {
                where_clauses.push(format!("date < date(\"{e}\")"));
            }
        }
    }

    // Amount filters
    for amount_str in &opts.amount {
        if let Ok(filter) = parse_amount_filter(amount_str) {
            let mut clause = format!("amount.number {} {}", filter.operator, filter.value);
            if let Some(cur) = &filter.currency {
                clause.push_str(&format!(" AND amount.currency = '{cur}'"));
            }
            where_clauses.push(clause);
        }
    }

    // Currency filter in WHERE clause (server-side)
    let currencies: Vec<String> = opts
        .currency
        .iter()
        .flat_map(|c| c.split(',').map(|s| s.trim().to_uppercase()))
        .filter(|s| !s.is_empty())
        .collect();
    match currencies.len() {
        0 => {}
        1 => where_clauses.push(format!("amount.currency = '{}'", currencies[0])),
        _ => {
            let list = currencies
                .iter()
                .map(|c| format!("'{c}'"))
                .collect::<Vec<_>>()
                .join(", ");
            where_clauses.push(format!("amount.currency IN ({list})"));
        }
    }

    let mut query = "SELECT date, account, amount FROM #balances".to_string();
    if !where_clauses.is_empty() {
        query.push_str(&format!(" WHERE {}", where_clauses.join(" AND ")));
    }

    // Sorting
    if let Some(sort_str) = &opts.sort {
        let sort_clause: Vec<String> = sort_str
            .split(',')
            .map(|field| {
                let field = field.trim();
                let (name, dir) = if let Some(stripped) = field.strip_prefix('-') {
                    (stripped, "DESC")
                } else {
                    (field, "ASC")
                };
                let mapped = match name {
                    "balance" => "amount",
                    other => other,
                };
                format!("{mapped} {dir}")
            })
            .collect();
        query.push_str(&format!(" ORDER BY {}", sort_clause.join(", ")));
    }

    if let Some(limit) = opts.limit {
        query.push_str(&format!(" LIMIT {limit}"));
    }

    query
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

fn parse_rows(json_rows: &[Value]) -> Result<Vec<AssertRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for row in json_rows {
        let date = row["date"]
            .as_str()
            .ok_or("missing date field")?
            .to_string();
        let account = row["account"]
            .as_str()
            .ok_or("missing account field")?
            .to_string();

        let amount = &row["amount"];
        let balance_currency = amount["currency"]
            .as_str()
            .ok_or("missing currency in amount")?
            .to_string();
        let number_str = amount["number"]
            .as_str()
            .ok_or("missing number in amount")?;
        let balance_number = number_str
            .parse::<Decimal>()
            .map_err(|_| format!("invalid decimal: {number_str}"))?;

        rows.push(AssertRow {
            date,
            account,
            balance_number,
            balance_currency,
        });
    }
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn format_balance(amount: Decimal, currency: &str) -> String {
    format!("{} {currency}", amount.normalize())
}

fn print_table(rows: &[AssertRow]) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);

    table.set_header(vec![
        Cell::new("Date").set_alignment(CellAlignment::Left),
        Cell::new("Account").set_alignment(CellAlignment::Left),
        Cell::new("Balance").set_alignment(CellAlignment::Right),
    ]);

    for row in rows {
        table.add_row(vec![
            Cell::new(&row.date).set_alignment(CellAlignment::Left),
            Cell::new(&row.account).set_alignment(CellAlignment::Left),
            Cell::new(format_balance(row.balance_number, &row.balance_currency))
                .set_alignment(CellAlignment::Right),
        ]);
    }

    println!("{table}");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CommonOptions;

    fn default_opts() -> CommonOptions {
        CommonOptions::default()
    }

    #[test]
    fn build_query_no_filters() {
        let q = build_query(&default_opts());
        assert_eq!(q, "SELECT date, account, amount FROM #balances");
    }

    #[test]
    fn build_query_account_filter() {
        let opts = CommonOptions {
            account: vec!["Assets:Bank".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("WHERE account ~ 'Assets:Bank'"));
    }

    #[test]
    fn build_query_account_exclusion() {
        let opts = CommonOptions {
            account: vec!["not".to_string(), "Assets:Cash".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("NOT (account ~ 'Assets:Cash')"));
    }

    #[test]
    fn build_query_date_filters() {
        let opts = CommonOptions {
            begin: Some("2025-11-01".to_string()),
            end: Some("2025-12-01".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("date >= date(\"2025-11-01\")"));
        assert!(q.contains("date < date(\"2025-12-01\")"));
    }

    #[test]
    fn build_query_currency_filter_single() {
        let opts = CommonOptions {
            currency: vec!["EUR".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("amount.currency = 'EUR'"));
    }

    #[test]
    fn build_query_currency_filter_multiple() {
        let opts = CommonOptions {
            currency: vec!["EUR,USD".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("amount.currency IN ('EUR', 'USD')"));
    }

    #[test]
    fn build_query_amount_filter() {
        let opts = CommonOptions {
            amount: vec![">500".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("amount.number > 500"));
    }

    #[test]
    fn build_query_amount_filter_with_currency() {
        let opts = CommonOptions {
            amount: vec![">500EUR".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("amount.number > 500 AND amount.currency = 'EUR'"));
    }

    #[test]
    fn build_query_sorting() {
        let opts = CommonOptions {
            sort: Some("account".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("ORDER BY account ASC"));
    }

    #[test]
    fn build_query_sorting_desc() {
        let opts = CommonOptions {
            sort: Some("-date".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("ORDER BY date DESC"));
    }

    #[test]
    fn build_query_sort_balance_maps_to_amount() {
        let opts = CommonOptions {
            sort: Some("balance".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("ORDER BY amount ASC"));
    }

    #[test]
    fn build_query_limit() {
        let opts = CommonOptions {
            limit: Some(2),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.ends_with("LIMIT 2"));
    }

    #[test]
    fn build_query_combined_filters() {
        let opts = CommonOptions {
            account: vec!["Assets:Bank".to_string()],
            amount: vec![">500".to_string()],
            currency: vec!["EUR".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("account ~ 'Assets:Bank'"));
        assert!(q.contains("amount.number > 500"));
        assert!(q.contains("amount.currency = 'EUR'"));
    }

    #[test]
    fn parse_rows_happy_path() {
        let json: serde_json::Value = serde_json::json!([{
            "date": "2025-11-07",
            "account": "Assets:Bank:Checking",
            "amount": { "number": "595.47", "currency": "EUR" }
        }]);
        let rows = parse_rows(json.as_array().unwrap()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2025-11-07");
        assert_eq!(rows[0].account, "Assets:Bank:Checking");
        assert_eq!(rows[0].balance_number, "595.47".parse::<Decimal>().unwrap());
        assert_eq!(rows[0].balance_currency, "EUR");
    }

    #[test]
    fn format_balance_strips_trailing_zeros() {
        assert_eq!(format_balance("595.47".parse().unwrap(), "EUR"), "595.47 EUR");
        assert_eq!(format_balance("100.00".parse().unwrap(), "USD"), "100 USD");
        assert_eq!(
            format_balance("5775.09".parse().unwrap(), "EUR"),
            "5775.09 EUR"
        );
    }
}
