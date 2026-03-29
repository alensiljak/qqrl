use comfy_table::{presets, Cell, CellAlignment, Table};
use rust_decimal::Decimal;
use serde_json::Value;

use crate::{
    cli::CommonOptions,
    config::Config,
    date_parser::{parse_date, parse_date_range},
    runner::run_bql_query,
    utils::parse_amount_filter,
};

#[derive(Debug, Clone)]
struct PriceRow {
    date: String,
    commodity: String,
    price_number: Decimal,
    price_currency: String,
}

/// Price history
///
/// Display commodity price history from the ledger file.
///
/// Usage:
///   qqrl price [COMMODITY] [OPTIONS]
///   qqrl p EUR
///   qqrl price --begin 2025-01-01 USD
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(opts.ledger.clone())?;
    let query = build_query(&opts);
    println!("\nYour BQL query is:\n{query}\n");
    let rows = run_bql_query(&config, &query)?;
    let mut price_rows = parse_rows(&rows)?;

    // Client-side currency filter: keeps only rows where the price is expressed in
    // one of the requested currencies (e.g. `-c USD` shows prices denominated in USD).
    let currencies: Vec<String> = opts
        .currency
        .iter()
        .flat_map(|c| c.split(',').map(|s| s.trim().to_uppercase()))
        .filter(|s| !s.is_empty())
        .collect();
    if !currencies.is_empty() {
        price_rows.retain(|r| currencies.contains(&r.price_currency));
    }

    print_table(&price_rows);
    Ok(())
}

// ---------------------------------------------------------------------------
// Query builder
// ---------------------------------------------------------------------------

fn build_query(opts: &CommonOptions) -> String {
    let mut where_clauses: Vec<String> = Vec::new();

    // Positional args are commodity filters.
    // Multiple commodities use OR logic (e.g. `qqrl p EUR USD` shows both).
    let commodities: Vec<String> = opts.account.iter().map(|s| s.to_uppercase()).collect();
    match commodities.len() {
        0 => {}
        1 => where_clauses.push(format!("currency ~ '{}'", commodities[0])),
        _ => {
            let conditions: Vec<String> =
                commodities.iter().map(|c| format!("currency ~ '{c}'")).collect();
            where_clauses.push(format!("({})", conditions.join(" OR ")));
        }
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

    // Amount filters (e.g. -a ">1.2" or -a ">1.2EUR")
    for amount_str in &opts.amount {
        if let Ok(filter) = parse_amount_filter(amount_str) {
            let mut clause = format!("amount.number {} {}", filter.operator, filter.value);
            if let Some(cur) = &filter.currency {
                clause.push_str(&format!(" AND amount.currency = '{cur}'"));
            }
            where_clauses.push(clause);
        }
    }

    let mut query = "SELECT date, currency, amount FROM #prices".to_string();
    if !where_clauses.is_empty() {
        query.push_str(&format!(" WHERE {}", where_clauses.join(" AND ")));
    }

    // Sort: explicit -S flag overrides the default (date, then commodity name).
    // Friendly names like "symbol" and "price" are mapped to BQL column names.
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
                    "symbol" => "currency",
                    "price" | "amount" => "amount",
                    other => other,
                };
                format!("{mapped} {dir}")
            })
            .collect();
        query.push_str(&format!(" ORDER BY {}", sort_clause.join(", ")));
    } else {
        query.push_str(" ORDER BY date, currency");
    }

    if let Some(limit) = opts.limit {
        query.push_str(&format!(" LIMIT {limit}"));
    }

    query
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

fn parse_rows(json_rows: &[Value]) -> Result<Vec<PriceRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for row in json_rows {
        let date = row["date"]
            .as_str()
            .ok_or("missing date field")?
            .to_string();
        let commodity = row["currency"]
            .as_str()
            .ok_or("missing currency field")?
            .to_string();

        let amount = &row["amount"];
        let price_currency = amount["currency"]
            .as_str()
            .ok_or("missing currency in amount")?
            .to_string();
        let number_str = amount["number"]
            .as_str()
            .ok_or("missing number in amount")?;
        let price_number = number_str
            .parse::<Decimal>()
            .map_err(|_| format!("invalid decimal: {number_str}"))?;

        rows.push(PriceRow {
            date,
            commodity,
            price_number,
            price_currency,
        });
    }
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn format_price(amount: Decimal, currency: &str) -> String {
    // normalize() strips trailing zeros (e.g. "1.05000" → "1.05") while
    // preserving the full precision that was stored in the ledger.
    format!("{} {currency}", amount.normalize())
}

fn print_table(rows: &[PriceRow]) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);

    table.set_header(vec![
        Cell::new("Date").set_alignment(CellAlignment::Left),
        Cell::new("Commodity").set_alignment(CellAlignment::Left),
        Cell::new("Price").set_alignment(CellAlignment::Right),
    ]);

    for row in rows {
        table.add_row(vec![
            Cell::new(&row.date).set_alignment(CellAlignment::Left),
            Cell::new(&row.commodity).set_alignment(CellAlignment::Left),
            Cell::new(format_price(row.price_number, &row.price_currency))
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
        assert_eq!(q, "SELECT date, currency, amount FROM #prices ORDER BY date, currency");
    }

    #[test]
    fn build_query_single_commodity() {
        let opts = CommonOptions {
            account: vec!["EUR".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("WHERE currency ~ 'EUR'"));
    }

    #[test]
    fn build_query_commodity_lowercased_is_uppercased() {
        let opts = CommonOptions {
            account: vec!["eur".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("currency ~ 'EUR'"));
        assert!(!q.contains("currency ~ 'eur'"));
    }

    #[test]
    fn build_query_multiple_commodities_uses_or() {
        let opts = CommonOptions {
            account: vec!["EUR".to_string(), "USD".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("(currency ~ 'EUR' OR currency ~ 'USD')"));
    }

    #[test]
    fn build_query_date_filters() {
        let opts = CommonOptions {
            begin: Some("2025-01-01".to_string()),
            end: Some("2025-12-31".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("date >= date(\"2025-01-01\")"));
        assert!(q.contains("date < date(\"2025-12-31\")"));
    }

    #[test]
    fn build_query_custom_sort() {
        let opts = CommonOptions {
            sort: Some("-date".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("ORDER BY date DESC"));
        assert!(!q.contains("ORDER BY date, currency"));
    }

    #[test]
    fn build_query_limit() {
        let opts = CommonOptions {
            limit: Some(10),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.ends_with("LIMIT 10"));
    }

    #[test]
    fn parse_rows_happy_path() {
        let json: serde_json::Value = serde_json::json!([{
            "date": "2025-03-15",
            "currency": "EUR",
            "amount": { "number": "1.0523", "currency": "USD" }
        }]);
        let rows = parse_rows(json.as_array().unwrap()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2025-03-15");
        assert_eq!(rows[0].commodity, "EUR");
        assert_eq!(rows[0].price_currency, "USD");
        assert_eq!(rows[0].price_number, "1.0523".parse::<Decimal>().unwrap());
    }

    #[test]
    fn format_price_strips_trailing_zeros() {
        assert_eq!(
            format_price("1.05000".parse().unwrap(), "USD"),
            "1.05 USD"
        );
        assert_eq!(
            format_price("150.00".parse().unwrap(), "USD"),
            "150 USD"
        );
        assert_eq!(
            format_price("1.0523".parse().unwrap(), "USD"),
            "1.0523 USD"
        );
    }

    #[test]
    fn build_query_amount_filter() {
        let opts = CommonOptions {
            amount: vec![">1.2".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("amount.number > 1.2"));
    }

    #[test]
    fn build_query_amount_filter_with_currency() {
        let opts = CommonOptions {
            amount: vec![">1.2EUR".to_string()],
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("amount.number > 1.2 AND amount.currency = 'EUR'"));
    }

    #[test]
    fn build_query_sort_friendly_names() {
        let opts = CommonOptions {
            sort: Some("symbol,-price".to_string()),
            ..default_opts()
        };
        let q = build_query(&opts);
        assert!(q.contains("ORDER BY currency ASC, amount DESC"));
    }

    #[test]
    fn currency_filter_applied_client_side() {
        // parse_rows returns both USD and EUR priced entries;
        // the -c USD filter should keep only the USD one.
        let json: serde_json::Value = serde_json::json!([
            { "date": "2025-01-01", "currency": "AAPL", "amount": { "number": "150.00", "currency": "USD" } },
            { "date": "2025-01-01", "currency": "AAPL", "amount": { "number": "138.50", "currency": "EUR" } }
        ]);
        let mut rows = parse_rows(json.as_array().unwrap()).unwrap();
        let filter = vec!["USD".to_string()];
        rows.retain(|r| filter.contains(&r.price_currency));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].price_currency, "USD");
    }
}
