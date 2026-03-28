use std::collections::HashMap;

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
struct Position {
    currency: String,
    amount: Decimal,
}

#[derive(Debug, Clone)]
struct RegisterRow {
    date: String,
    account: String,
    payee: String,
    narration: String,
    amount: Decimal,
    currency: String,
    converted_amount: Option<Position>,
}

/// Transaction register command
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(opts.ledger.clone())?;
    let query = build_query(&opts);
    println!("\nYour BQL query is:\n{query}\n");
    let rows = run_bql_query(&config, &query)?;
    let register_rows = parse_rows(&rows)?;
    // Capitalize exchange value for display
    let exchange_display = opts.exchange.as_ref().map(|s| s.to_uppercase());
    print_table(&register_rows, opts.total, exchange_display.as_deref());
    Ok(())
}

// ---------------------------------------------------------------------------
// Query builder
// ---------------------------------------------------------------------------

fn build_query(opts: &CommonOptions) -> String {
    let mut where_clauses: Vec<String> = Vec::new();

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

    // Currency filter — split comma-separated values, support -c EUR -c USD
    // Convert to uppercase for case-insensitive matching
    let currencies: Vec<String> = opts
        .currency
        .iter()
        .flat_map(|c| c.split(',').map(|s| s.trim().to_uppercase()))
        .filter(|s| !s.is_empty())
        .collect();
    if currencies.len() == 1 {
        where_clauses.push(format!("currency = '{}'", currencies[0]));
    } else if currencies.len() > 1 {
        let conditions: Vec<String> = currencies.iter().map(|c| format!("currency = '{c}'")).collect();
        where_clauses.push(format!("({})", conditions.join(" OR ")));
    }

    // Amount filters go directly into WHERE for register (unlike balance post-filtering)
    for amount_str in &opts.amount {
        if let Ok(filter) = parse_amount_filter(amount_str) {
            let mut clause = format!("number {} {}", filter.operator, filter.value);
            if let Some(cur) = &filter.currency {
                clause.push_str(&format!(" AND currency = '{cur}'"));
            }
            where_clauses.push(clause);
        }
    }

    let mut query = if let Some(exchange) = &opts.exchange {
        let exchange_upper = exchange.to_uppercase();
        format!(
            "SELECT date, account, payee, narration, position, convert(position, '{exchange_upper}') as Converted"
        )
    } else {
        "SELECT date, account, payee, narration, position".to_string()
    };
    if !where_clauses.is_empty() {
        query.push_str(&format!(" WHERE {}", where_clauses.join(" AND ")));
    }

    // Only apply ORDER BY when user explicitly requests a sort
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
                format!("{name} {dir}")
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

fn parse_rows(json_rows: &[Value]) -> Result<Vec<RegisterRow>, Box<dyn std::error::Error>> {
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
        let payee = row["payee"].as_str().unwrap_or("").to_string();
        let narration = row["narration"].as_str().unwrap_or("").to_string();

        let units = &row["position"]["units"];
        let currency = units["currency"]
            .as_str()
            .ok_or("missing currency in position")?
            .to_string();
        let number_str = units["number"]
            .as_str()
            .ok_or("missing number in position")?;
        let amount = number_str
            .parse::<Decimal>()
            .map_err(|_| format!("invalid decimal: {number_str}"))?;

        let converted_amount = row.get("Converted").map(parse_position).transpose()?;

        rows.push(RegisterRow {
            date,
            account,
            payee,
            narration,
            amount,
            currency,
            converted_amount,
        });
    }
    Ok(rows)
}

fn parse_position(value: &Value) -> Result<Position, Box<dyn std::error::Error>> {
    let currency = value["currency"]
        .as_str()
        .ok_or("missing currency in converted position")?
        .to_string();
    let number_str = value["number"]
        .as_str()
        .ok_or("missing number in converted position")?;
    let amount = number_str
        .parse::<Decimal>()
        .map_err(|_| format!("invalid decimal: {number_str}"))?;

    Ok(Position { currency, amount })
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn format_amount(amount: Decimal, currency: &str) -> String {
    let rounded = amount.round_dp(2);
    let s = format!("{rounded:.2}");

    let (int_part, frac_part) = s.split_once('.').unwrap_or((&s, ""));
    let (negative, digits) = if let Some(stripped) = int_part.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, int_part)
    };

    let with_commas = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",");

    let sign = if negative { "-" } else { "" };
    format!("{sign}{with_commas}.{frac_part} {currency}")
}

fn format_running_totals(totals: &HashMap<String, Decimal>) -> String {
    let mut parts: Vec<String> = totals
        .iter()
        .map(|(cur, amt)| format_amount(*amt, cur))
        .collect();
    parts.sort();
    parts.join(" ")
}

fn print_table(rows: &[RegisterRow], show_total: bool, exchange: Option<&str>) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);

    let mut headers = vec![
        Cell::new("Date").set_alignment(CellAlignment::Left),
        Cell::new("Account").set_alignment(CellAlignment::Left),
        Cell::new("Payee").set_alignment(CellAlignment::Left),
        Cell::new("Narration").set_alignment(CellAlignment::Left),
        Cell::new("Amount").set_alignment(CellAlignment::Right),
    ];
    if show_total {
        headers.push(Cell::new("Running Total").set_alignment(CellAlignment::Right));
    }
    if let Some(currency) = exchange {
        headers.push(Cell::new(format!("Amount ({currency})")).set_alignment(CellAlignment::Right));
        if show_total {
            headers
                .push(Cell::new(format!("Total ({currency})")).set_alignment(CellAlignment::Right));
        }
    }
    table.set_header(headers);

    let mut running_totals: HashMap<String, Decimal> = HashMap::new();
    let mut converted_running_total = Decimal::ZERO;

    for row in rows {
        *running_totals.entry(row.currency.clone()).or_default() += row.amount;
        if let Some(converted_amount) = &row.converted_amount {
            converted_running_total += converted_amount.amount;
        }

        let mut cells = vec![
            Cell::new(&row.date).set_alignment(CellAlignment::Left),
            Cell::new(&row.account).set_alignment(CellAlignment::Left),
            Cell::new(&row.payee).set_alignment(CellAlignment::Left),
            Cell::new(&row.narration).set_alignment(CellAlignment::Left),
            Cell::new(format_amount(row.amount, &row.currency)).set_alignment(CellAlignment::Right),
        ];
        if show_total {
            cells.push(
                Cell::new(format_running_totals(&running_totals))
                    .set_alignment(CellAlignment::Right),
            );
        }
        if let Some(currency) = exchange {
            let converted_value = row
                .converted_amount
                .as_ref()
                .map(|position| format_amount(position.amount, &position.currency))
                .unwrap_or_default();
            cells.push(Cell::new(converted_value).set_alignment(CellAlignment::Right));
            if show_total {
                cells.push(
                    Cell::new(format_amount(converted_running_total, currency))
                        .set_alignment(CellAlignment::Right),
                );
            }
        }
        table.add_row(cells);
    }

    println!("{table}");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_amount_positive() {
        assert_eq!(format_amount("20.00".parse().unwrap(), "EUR"), "20.00 EUR");
        assert_eq!(
            format_amount("1000.00".parse().unwrap(), "EUR"),
            "1,000.00 EUR"
        );
    }

    #[test]
    fn format_amount_negative() {
        assert_eq!(
            format_amount("-100.00".parse().unwrap(), "EUR"),
            "-100.00 EUR"
        );
    }

    #[test]
    fn parse_rows_happy_path() {
        let json: Value = serde_json::json!([{
            "date": "2025-03-15",
            "account": "Assets:Bank:Checking",
            "payee": "Employer",
            "narration": "Salary",
            "position": {
                "cost": null,
                "units": { "currency": "EUR", "number": "1000" }
            }
        }]);
        let rows = parse_rows(json.as_array().unwrap()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2025-03-15");
        assert_eq!(rows[0].account, "Assets:Bank:Checking");
        assert_eq!(rows[0].payee, "Employer");
        assert_eq!(rows[0].narration, "Salary");
        assert_eq!(rows[0].currency, "EUR");
        assert_eq!(rows[0].amount, "1000".parse::<Decimal>().unwrap());
    }

    #[test]
    fn build_query_with_exchange_lowercase_is_capitalized() {
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: Some("usd".to_string()),  // lowercase
            sort: None,
            limit: None,
            total: false,
            no_pager: false,
            hierarchy: false,
            empty: false,
            depth: None,
            zero: false,
            ledger: None,
            list: false,
        };

        let q = build_query(&opts);
        // Should be capitalized in the query
        assert!(q.contains("convert(position, 'USD')"));
        assert!(!q.contains("convert(position, 'usd')"));
    }

    #[test]
    fn build_query_with_currency_lowercase_is_capitalized() {
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec!["usd".to_string()],  // lowercase
            exchange: None,
            sort: None,
            limit: None,
            total: false,
            no_pager: false,
            hierarchy: false,
            empty: false,
            depth: None,
            zero: false,
            ledger: None,
            list: false,
        };

        let q = build_query(&opts);
        // Should be capitalized in the query
        assert!(q.contains("currency = 'USD'"));
        assert!(!q.contains("currency = 'usd'"));

        // Test with multiple currencies mixed case
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec!["Eur,gbp".to_string()],  // mixed case
            exchange: None,
            sort: None,
            limit: None,
            total: false,
            no_pager: false,
            hierarchy: false,
            empty: false,
            depth: None,
            zero: false,
            ledger: None,
            list: false,
        };

        let q = build_query(&opts);
        assert!(q.contains("currency = 'EUR' OR currency = 'GBP'"));
        assert!(!q.contains("currency = 'Eur'"));
        assert!(!q.contains("currency = 'gbp'"));
    }

    #[test]
    fn parse_rows_null_payee() {
        let json: Value = serde_json::json!([{
            "date": "2025-01-01",
            "account": "Assets:Bank:Checking",
            "payee": null,
            "narration": "Initial Balance",
            "position": {
                "cost": null,
                "units": { "currency": "EUR", "number": "1000" }
            }
        }]);
        let rows = parse_rows(json.as_array().unwrap()).unwrap();
        assert_eq!(rows[0].payee, "");
    }
}
