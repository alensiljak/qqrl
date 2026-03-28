use std::collections::{BTreeMap, HashMap};

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
struct BalanceRow {
    account: String,
    positions: Vec<Position>,
    converted_positions: Option<Vec<Position>>,
}

#[derive(Debug, Clone)]
struct BalanceTotals {
    positions: Vec<Position>,
    converted_positions: Option<Vec<Position>>,
}

/// Account balances command
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(opts.ledger.clone())?;
    let query = build_query(&opts);
    println!("\nYour BQL query is:\n{query}\n");
    let rows = run_bql_query(&config, &query)?;

    let mut balance_rows = parse_rows(&rows)?;

    // Apply amount filters post-query (on aggregated balances)
    if !opts.amount.is_empty() {
        balance_rows = apply_amount_filters(balance_rows, &opts.amount)?;
    }

    // Hierarchy expands parent accounts with aggregated balances
    if opts.hierarchy {
        balance_rows = apply_hierarchy(balance_rows);
    }

    // Depth: collapse deeper accounts into their depth-level ancestor
    if let Some(depth) = opts.depth {
        if !opts.hierarchy {
            // Without hierarchy: collapse leaf accounts deeper than N
            balance_rows = apply_depth_collapse(balance_rows, depth);
        }
        // With hierarchy: just filter (parents already carry aggregated balances)
        balance_rows.retain(|r| r.account.matches(':').count() < depth as usize);
    }

    // Exclude zero-balance accounts
    if opts.zero {
        balance_rows.retain(|r| {
            !r.positions.is_empty() && r.positions.iter().any(|p| p.amount != Decimal::ZERO)
        });
    }

    // Compute grand total before writing output
    let grand_total = if opts.total {
        compute_grand_total(&balance_rows, opts.hierarchy, opts.depth)
    } else {
        None
    };

    // Capitalize exchange value for display
    let exchange_display = opts.exchange.as_ref().map(|s| s.to_uppercase());
    print_table(
        &balance_rows,
        grand_total.as_ref(),
        exchange_display.as_deref(),
    );
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

    let select_clause = if let Some(exchange) = &opts.exchange {
        let exchange_upper = exchange.to_uppercase();
        format!(
            "SELECT account, units(sum(position)) as Balance, sum(convert(position, '{exchange_upper}')) as Converted"
        )
    } else {
        "SELECT account, units(sum(position)) as Balance".to_string()
    };

    let mut query = format!("{select_clause} GROUP BY account");
    if !where_clauses.is_empty() {
        query = format!(
            "{select_clause} WHERE {} GROUP BY account",
            where_clauses.join(" AND ")
        );
    }

    // Sort — default is account ASC
    let sort_str = opts.sort.as_deref().unwrap_or("account");
    let sort_clause: Vec<String> = sort_str
        .split(',')
        .map(|field| {
            let field = field.trim();
            let (name, dir) = if let Some(stripped) = field.strip_prefix('-') {
                (stripped, "DESC")
            } else {
                (field, "ASC")
            };
            let bql_field = if name == "balance" {
                "sum(position)".to_string()
            } else {
                name.to_string()
            };
            format!("{bql_field} {dir}")
        })
        .collect();
    query.push_str(&format!(" ORDER BY {}", sort_clause.join(", ")));

    if let Some(limit) = opts.limit {
        query.push_str(&format!(" LIMIT {limit}"));
    }

    query
}

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

fn parse_rows(json_rows: &[Value]) -> Result<Vec<BalanceRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for row in json_rows {
        let account = row["account"]
            .as_str()
            .ok_or("missing account field")?
            .to_string();

        let positions = parse_inventory_positions(&row["Balance"], "Balance")?;
        let converted_positions = row
            .get("Converted")
            .map(|value| parse_inventory_positions(value, "Converted"))
            .transpose()?;

        rows.push(BalanceRow {
            account,
            positions,
            converted_positions,
        });
    }
    Ok(rows)
}

fn parse_inventory_positions(
    inventory: &Value,
    label: &str,
) -> Result<Vec<Position>, Box<dyn std::error::Error>> {
    let positions_json = inventory["positions"]
        .as_array()
        .ok_or_else(|| format!("missing positions array in {label}"))?;

    let mut positions = Vec::new();
    for pos in positions_json {
        let currency = pos["currency"]
            .as_str()
            .ok_or("missing currency in position")?
            .to_string();
        let number_str = pos["number"].as_str().ok_or("missing number in position")?;
        let amount = number_str
            .parse::<Decimal>()
            .map_err(|_| format!("invalid decimal: {number_str}"))?;
        positions.push(Position { currency, amount });
    }

    Ok(positions)
}

// ---------------------------------------------------------------------------
// Post-processing
// ---------------------------------------------------------------------------

fn apply_amount_filters(
    rows: Vec<BalanceRow>,
    amount_args: &[String],
) -> Result<Vec<BalanceRow>, Box<dyn std::error::Error>> {
    let filters: Vec<_> = amount_args
        .iter()
        .map(|a| parse_amount_filter(a))
        .collect::<Result<Vec<_>, _>>()?;

    let filtered = rows
        .into_iter()
        .filter(|row| {
            filters.iter().all(|filter| {
                row.positions.iter().any(|pos| {
                    // Skip if currency doesn't match the filter's expected currency
                    if let Some(ref cur) = filter.currency {
                        if pos.currency != *cur {
                            return false;
                        }
                    }
                    match filter.operator.as_str() {
                        ">" => pos.amount > filter.value,
                        ">=" => pos.amount >= filter.value,
                        "<" => pos.amount < filter.value,
                        "<=" => pos.amount <= filter.value,
                        "=" => pos.amount == filter.value,
                        _ => false,
                    }
                })
            })
        })
        .collect();

    Ok(filtered)
}

/// Expand output to include parent accounts, each carrying the sum of all children.
fn apply_hierarchy(rows: Vec<BalanceRow>) -> Vec<BalanceRow> {
    // accumulate: account_name -> currency -> amount
    let mut totals: BTreeMap<String, HashMap<String, Decimal>> = BTreeMap::new();
    let mut converted_totals: BTreeMap<String, HashMap<String, Decimal>> = BTreeMap::new();

    for row in &rows {
        let parts: Vec<&str> = row.account.split(':').collect();
        // Add this row's positions into every ancestor level (including itself)
        for depth in 1..=parts.len() {
            let account = parts[..depth].join(":");
            let entry = totals.entry(account.clone()).or_default();
            for pos in &row.positions {
                *entry.entry(pos.currency.clone()).or_default() += pos.amount;
            }

            if let Some(converted_positions) = &row.converted_positions {
                let converted_entry = converted_totals.entry(account).or_default();
                for pos in converted_positions {
                    *converted_entry.entry(pos.currency.clone()).or_default() += pos.amount;
                }
            }
        }
    }

    totals
        .into_iter()
        .map(|(account, currencies)| {
            let positions = positions_from_currency_map(currencies);
            let converted_positions = converted_totals
                .remove(&account)
                .map(positions_from_currency_map);
            BalanceRow {
                account,
                positions,
                converted_positions,
            }
        })
        .collect()
}

/// Collapse leaf accounts deeper than `depth` into their `depth`-level ancestor.
fn apply_depth_collapse(rows: Vec<BalanceRow>, depth: u32) -> Vec<BalanceRow> {
    let mut collapsed: BTreeMap<String, HashMap<String, Decimal>> = BTreeMap::new();
    let mut collapsed_converted: BTreeMap<String, HashMap<String, Decimal>> = BTreeMap::new();

    for row in &rows {
        let parts: Vec<&str> = row.account.split(':').collect();
        let ancestor = if parts.len() > depth as usize {
            parts[..depth as usize].join(":")
        } else {
            row.account.clone()
        };

        let entry = collapsed.entry(ancestor).or_default();
        for pos in &row.positions {
            *entry.entry(pos.currency.clone()).or_default() += pos.amount;
        }

        if let Some(converted_positions) = &row.converted_positions {
            let converted_entry = collapsed_converted.entry(
                row.account
                    .split(':')
                    .collect::<Vec<_>>()
                    .into_iter()
                    .take(depth as usize)
                    .collect::<Vec<_>>()
                    .join(":"),
            );
            let converted_entry = converted_entry.or_default();
            for pos in converted_positions {
                *converted_entry.entry(pos.currency.clone()).or_default() += pos.amount;
            }
        }
    }

    collapsed
        .into_iter()
        .map(|(account, currencies)| {
            let positions = positions_from_currency_map(currencies);
            let converted_positions = collapsed_converted
                .remove(&account)
                .map(positions_from_currency_map);
            BalanceRow {
                account,
                positions,
                converted_positions,
            }
        })
        .collect()
}

fn positions_from_currency_map(currencies: HashMap<String, Decimal>) -> Vec<Position> {
    let mut positions: Vec<Position> = currencies
        .into_iter()
        .map(|(currency, amount)| Position { currency, amount })
        .collect();
    positions.sort_by(|a, b| a.currency.cmp(&b.currency));
    positions
}

// ---------------------------------------------------------------------------
// Grand total
// ---------------------------------------------------------------------------

/// Compute grand totals, avoiding double-counting when hierarchy is active.
fn compute_grand_total(
    rows: &[BalanceRow],
    hierarchy: bool,
    depth: Option<u32>,
) -> Option<BalanceTotals> {
    if rows.is_empty() {
        return None;
    }

    // With hierarchy, sum only top-level (minimum depth) accounts to avoid double-counting
    let root_depth = if hierarchy {
        rows.iter()
            .map(|r| r.account.matches(':').count() + 1)
            .min()
            .unwrap_or(1)
    } else {
        0 // ignored when hierarchy = false
    };

    let mut totals: HashMap<String, Decimal> = HashMap::new();
    let mut converted_totals: HashMap<String, Decimal> = HashMap::new();
    for row in rows {
        let include = if hierarchy {
            row.account.matches(':').count() + 1 == root_depth
        } else if let Some(d) = depth {
            // depth collapse already made all rows at the right depth
            row.account.matches(':').count() < d as usize
        } else {
            true
        };

        if include {
            for pos in &row.positions {
                *totals.entry(pos.currency.clone()).or_default() += pos.amount;
            }
            if let Some(converted_positions) = &row.converted_positions {
                for pos in converted_positions {
                    *converted_totals.entry(pos.currency.clone()).or_default() += pos.amount;
                }
            }
        }
    }

    if totals.is_empty() {
        return None;
    }

    Some(BalanceTotals {
        positions: positions_from_currency_map(totals),
        converted_positions: if converted_totals.is_empty() {
            None
        } else {
            Some(positions_from_currency_map(converted_totals))
        },
    })
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn format_positions(positions: &[Position]) -> String {
    positions
        .iter()
        .map(|p| format_amount(p.amount, &p.currency))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_amount(amount: Decimal, currency: &str) -> String {
    // Format with 2 decimal places and thousands separators
    let rounded = amount.round_dp(2);
    let s = format!("{rounded:.2}");

    // Insert thousands separators
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

fn print_table(rows: &[BalanceRow], grand_total: Option<&BalanceTotals>, exchange: Option<&str>) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    let mut headers = vec![
        Cell::new("Account").set_alignment(CellAlignment::Left),
        Cell::new("Balance").set_alignment(CellAlignment::Right),
    ];
    if let Some(currency) = exchange {
        headers.push(Cell::new(format!("Total ({currency})")).set_alignment(CellAlignment::Right));
    }
    table.set_header(headers);

    for row in rows {
        let mut cells = vec![
            Cell::new(&row.account).set_alignment(CellAlignment::Left),
            Cell::new(format_positions(&row.positions)).set_alignment(CellAlignment::Right),
        ];
        if exchange.is_some() {
            cells.push(
                Cell::new(format_positions(
                    row.converted_positions.as_deref().unwrap_or(&[]),
                ))
                .set_alignment(CellAlignment::Right),
            );
        }
        table.add_row(cells);
    }

    if let Some(total_positions) = grand_total {
        let mut separator = vec![
            Cell::new("-------------------").set_alignment(CellAlignment::Left),
            Cell::new("-------------------").set_alignment(CellAlignment::Right),
        ];
        if exchange.is_some() {
            separator.push(Cell::new("-------------------").set_alignment(CellAlignment::Right));
        }
        table.add_row(separator);

        let mut total_row = vec![
            Cell::new("Total").set_alignment(CellAlignment::Left),
            Cell::new(format_positions(&total_positions.positions))
                .set_alignment(CellAlignment::Right),
        ];
        if exchange.is_some() {
            total_row.push(
                Cell::new(format_positions(
                    total_positions
                        .converted_positions
                        .as_deref()
                        .unwrap_or(&[]),
                ))
                .set_alignment(CellAlignment::Right),
            );
        }
        table.add_row(total_row);
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
    fn format_amount_basic() {
        assert_eq!(
            format_amount("1369.80".parse().unwrap(), "EUR"),
            "1,369.80 EUR"
        );
        assert_eq!(format_amount("20".parse().unwrap(), "EUR"), "20.00 EUR");
        assert_eq!(
            format_amount("-1000".parse().unwrap(), "EUR"),
            "-1,000.00 EUR"
        );
        assert_eq!(format_amount("-7".parse().unwrap(), "USD"), "-7.00 USD");
        assert_eq!(
            format_amount("1234567.89".parse().unwrap(), "CHF"),
            "1,234,567.89 CHF"
        );
    }

    #[test]
    fn build_query_defaults() {
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
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
        assert!(q.contains("SELECT account, units(sum(position)) as Balance"));
        assert!(q.contains("GROUP BY account"));
        assert!(q.contains("ORDER BY account ASC"));
        assert!(!q.contains("WHERE"));
    }

    #[test]
    fn build_query_with_account_filter() {
        let opts = CommonOptions {
            account: vec!["Assets".to_string()],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
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
        assert!(q.contains("account ~ 'Assets'"));
        assert!(q.contains("WHERE"));
    }

    #[test]
    fn build_query_with_date_range() {
        let opts = CommonOptions {
            account: vec![],
            begin: Some("2025-01".to_string()),
            end: Some("2025-09".to_string()),
            date_range: None,
            amount: vec![],
            currency: vec![],
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
        assert!(q.contains("date >= date(\"2025-01-01\")"));
        assert!(q.contains("date < date(\"2025-09-01\")"));
    }

    #[test]
    fn build_query_with_exchange_uses_sum_of_converted_positions() {
        let opts = CommonOptions {
            account: vec!["Assets:Bank:Bank03581".to_string()],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: Some("EUR".to_string()),
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

        assert!(q.contains("sum(convert(position, 'EUR')) as Converted"));
        assert!(!q.contains("convert(sum(position), 'EUR')"));
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
            exchange: Some("eur".to_string()),  // lowercase
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
        assert!(q.contains("convert(position, 'EUR')"));
        assert!(!q.contains("convert(position, 'eur')"));
    }

    #[test]
    fn build_query_with_currency_lowercase_is_capitalized() {
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec!["eur".to_string()],  // lowercase single currency
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
        assert!(q.contains("currency = 'EUR'"));
        assert!(!q.contains("currency = 'eur'"));

        // Test with multiple currencies, some lowercase
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec!["eur,usd".to_string()],  // comma-separated lowercase
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
        assert!(q.contains("currency = 'EUR' OR currency = 'USD'"));
        assert!(!q.contains("currency = 'eur'"));
        assert!(!q.contains("currency = 'usd'"));
    }

    #[test]
    fn build_query_sort_desc() {
        let opts = CommonOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: None,
            sort: Some("-account".to_string()),
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
        assert!(q.contains("ORDER BY account DESC"));
    }

    #[test]
    fn apply_hierarchy_aggregates_parents() {
        let rows = vec![
            BalanceRow {
                account: "Assets:Bank:Checking".to_string(),
                positions: vec![Position {
                    currency: "EUR".to_string(),
                    amount: "1000".parse().unwrap(),
                }],
                converted_positions: None,
            },
            BalanceRow {
                account: "Assets:Bank:Savings".to_string(),
                positions: vec![Position {
                    currency: "EUR".to_string(),
                    amount: "500".parse().unwrap(),
                }],
                converted_positions: None,
            },
        ];
        let result = apply_hierarchy(rows);

        let accounts: Vec<&str> = result.iter().map(|r| r.account.as_str()).collect();
        assert!(accounts.contains(&"Assets"));
        assert!(accounts.contains(&"Assets:Bank"));
        assert!(accounts.contains(&"Assets:Bank:Checking"));
        assert!(accounts.contains(&"Assets:Bank:Savings"));

        let assets = result.iter().find(|r| r.account == "Assets").unwrap();
        assert_eq!(
            assets.positions[0].amount,
            "1500".parse::<Decimal>().unwrap()
        );

        let bank = result.iter().find(|r| r.account == "Assets:Bank").unwrap();
        assert_eq!(bank.positions[0].amount, "1500".parse::<Decimal>().unwrap());
    }

    #[test]
    fn apply_depth_collapse_works() {
        let rows = vec![
            BalanceRow {
                account: "Assets:Bank:Checking".to_string(),
                positions: vec![Position {
                    currency: "EUR".to_string(),
                    amount: "1000".parse().unwrap(),
                }],
                converted_positions: None,
            },
            BalanceRow {
                account: "Assets:Bank:Savings".to_string(),
                positions: vec![Position {
                    currency: "EUR".to_string(),
                    amount: "500".parse().unwrap(),
                }],
                converted_positions: None,
            },
            BalanceRow {
                account: "Expenses:Food".to_string(),
                positions: vec![Position {
                    currency: "EUR".to_string(),
                    amount: "100".parse().unwrap(),
                }],
                converted_positions: None,
            },
        ];
        let result = apply_depth_collapse(rows, 1);

        assert_eq!(result.len(), 2);
        let assets = result.iter().find(|r| r.account == "Assets").unwrap();
        assert_eq!(
            assets.positions[0].amount,
            "1500".parse::<Decimal>().unwrap()
        );
    }
}
