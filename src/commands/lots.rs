use comfy_table::{presets, Cell, CellAlignment, Table};
use rust_decimal::Decimal;
use serde_json::Value;

use crate::{
    cli::LotsOptions,
    config::Config,
    date_parser::{parse_date, parse_date_range},
    runner::run_bql_query,
    utils::{parse_account_params, parse_account_pattern, parse_amount_filter},
};

#[derive(Debug, Clone)]
struct Quantity {
    amount: Decimal,
    //currency: String,
}

#[derive(Debug, Clone)]
struct Amount {
    currency: String,
    amount: Decimal,
}

#[derive(Debug, Clone)]
struct LotsRow {
    date: String,
    account: String,
    symbol: String,
    quantity: Quantity,
    price: Decimal,
    cost: Amount,
    value: Amount,
    converted_value: Option<Amount>,
}

#[derive(Debug, Clone)]
struct AverageLotsRow {
    date: String,
    account: String,
    symbol: String,
    quantity: Quantity,
    average_price: Decimal,
    total_cost: Amount,
    value: Amount,
    converted_value: Option<Amount>,
}

pub fn run(opts: LotsOptions) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(opts.ledger.clone())?;
    let query = build_query(&opts);
    let rows = run_bql_query(&config, &query)?;

    println!("\nYour BQL query is:\n{query}\n");
    if opts.average {
        let average_rows = parse_average_rows(&rows)?;
        let exchange_display = opts.exchange.as_ref().map(|s| s.to_uppercase());
        print_average_table(&average_rows, exchange_display.as_deref());
    } else {
        let lots_rows = parse_rows(&rows)?;
        let exchange_display = opts.exchange.as_ref().map(|s| s.to_uppercase());
        print_table(&lots_rows, exchange_display.as_deref());
    }

    Ok(())
}

fn build_query(opts: &LotsOptions) -> String {
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
            if let Some(begin) = begin {
                where_clauses.push(format!("date >= date(\"{begin}\")"));
            }
            if let Some(end) = end {
                where_clauses.push(format!("date < date(\"{end}\")"));
            }
        }
    }

    for amount_str in &opts.amount {
        if let Ok(filter) = parse_amount_filter(amount_str) {
            let mut clause = format!("number {} {}", filter.operator, filter.value);
            if let Some(currency) = filter.currency {
                clause.push_str(&format!(" AND currency = '{currency}'"));
            }
            where_clauses.push(clause);
        }
    }

    let currencies: Vec<String> = opts
        .currency
        .iter()
        .flat_map(|c| c.split(',').map(|part| part.trim().to_uppercase()))
        .filter(|c| !c.is_empty())
        .collect();
    if currencies.len() == 1 {
        where_clauses.push(format!("currency = '{}'", currencies[0]));
    } else if currencies.len() > 1 {
        let list = currencies.join("', '");
        where_clauses.push(format!("currency IN ('{list}')"));
    }

    where_clauses.push("cost_number IS NOT NULL".to_string());

    let (select_clause, group_by, having_clause) = if opts.average {
        let mut query = "SELECT MAX(date) as date, account, currency(units(position)) as symbol, SUM(units(position)) as quantity, SUM(cost_number * number(units(position))) / SUM(number(units(position))) as avg_price, cost(SUM(position)) as total_cost, value(SUM(position)) as value".to_string();
        if opts.exchange.is_some() {
            let exchange_upper = opts.exchange.as_ref().unwrap().to_uppercase();
            query.push_str(&format!(", convert(value(SUM(position)), '{exchange_upper}') as converted_value"));
        }
        (query, Some(vec!["account", "currency(units(position))"]), None)
    } else if opts.closed {
        let mut query = "SELECT MAX(date) as date, account, currency(units(position)) as symbol, SUM(units(position)) as quantity, cost_number as price, cost(SUM(position)) as cost, value(SUM(position)) as value".to_string();
        if opts.exchange.is_some() {
            let exchange_upper = opts.exchange.as_ref().unwrap().to_uppercase();
            query.push_str(&format!(", convert(value(SUM(position)), '{exchange_upper}') as converted_value"));
        }
        (query, Some(vec![
            "account",
            "currency(units(position))",
            "cost_number",
            "cost_currency",
        ]), Some("HAVING SUM(number(units(position))) <= 0".to_string()))
    } else if opts.show_all {
        let mut query = "SELECT date, account, currency(units(position)) as symbol, units(position) as quantity, cost_number as price, cost(position) as cost, value(position) as value".to_string();
        if opts.exchange.is_some() {
            let exchange_upper = opts.exchange.as_ref().unwrap().to_uppercase();
            query.push_str(&format!(", convert(value(position), '{exchange_upper}') as converted_value"));
        }
        (query, None, None)
    } else {
        let mut query = "SELECT MAX(date) as date, account, currency(units(position)) as symbol, SUM(units(position)) as quantity, cost_number as price, cost(SUM(position)) as cost, value(SUM(position)) as value".to_string();
        if opts.exchange.is_some() {
            let exchange_upper = opts.exchange.as_ref().unwrap().to_uppercase();
            query.push_str(&format!(", convert(value(SUM(position)), '{exchange_upper}') as converted_value"));
        }
        (query, Some(vec![
            "account",
            "currency(units(position))",
            "cost_number",
            "cost_currency",
        ]), Some("HAVING SUM(number(units(position))) > 0".to_string()))
    };

    let mut query = select_clause;
    if !where_clauses.is_empty() {
        query.push_str(&format!(" WHERE {}", where_clauses.join(" AND ")));
    }
    if let Some(group_by) = group_by {
        query.push_str(&format!(" GROUP BY {}", group_by.join(", ")));
    }
    if let Some(having) = having_clause {
        query.push(' ');
        query.push_str(&having);
    }

    let order_by = if let Some(sort) = &opts.sort {
        sort.split(',')
            .map(|field| {
                let field = field.trim();
                let (name, dir) = if let Some(stripped) = field.strip_prefix('-') {
                    (stripped, "DESC")
                } else {
                    (field, "ASC")
                };
                format!("{name} {dir}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else if let Some(sort_by) = &opts.sort_by {
        let field = match (opts.average, sort_by.as_str()) {
            (true, "price") => "avg_price",
            (_, other) => other,
        };
        format!("{field} ASC")
    } else {
        "date ASC".to_string()
    };
    query.push_str(&format!(" ORDER BY {order_by}"));

    if let Some(limit) = opts.limit {
        query.push_str(&format!(" LIMIT {limit}"));
    }

    query
}

fn parse_rows(json_rows: &[Value]) -> Result<Vec<LotsRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for row in json_rows {
        let converted_value = if row.get("converted_value").is_some() {
            Some(parse_amount(&row["converted_value"], "converted_value")?)
        } else {
            None
        };
        rows.push(LotsRow {
            date: required_str(row, "date")?.to_string(),
            account: required_str(row, "account")?.to_string(),
            symbol: required_str(row, "symbol")?.to_string(),
            quantity: parse_quantity(&row["quantity"])?,
            price: parse_decimal_value(&row["price"], "price")?,
            cost: parse_amount(&row["cost"], "cost")?,
            value: parse_amount(&row["value"], "value")?,
            converted_value,
        });
    }
    Ok(rows)
}

fn parse_average_rows(
    json_rows: &[Value],
) -> Result<Vec<AverageLotsRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    for row in json_rows {
        let converted_value = if row.get("converted_value").is_some() {
            Some(parse_amount(&row["converted_value"], "converted_value")?)
        } else {
            None
        };
        rows.push(AverageLotsRow {
            date: required_str(row, "date")?.to_string(),
            account: required_str(row, "account")?.to_string(),
            symbol: required_str(row, "symbol")?.to_string(),
            quantity: parse_quantity(&row["quantity"])?,
            average_price: parse_decimal_value(&row["avg_price"], "avg_price")?,
            total_cost: parse_amount(&row["total_cost"], "total_cost")?,
            value: parse_amount(&row["value"], "value")?,
            converted_value,
        });
    }
    Ok(rows)
}

fn required_str<'a>(row: &'a Value, field: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    row[field]
        .as_str()
        .ok_or_else(|| format!("missing {field} field").into())
}

fn parse_decimal_value(value: &Value, label: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
    let number_str = value
        .as_str()
        .ok_or_else(|| format!("missing {label} field"))?;
    number_str
        .parse::<Decimal>()
        .map_err(|_| format!("invalid decimal in {label}: {number_str}").into())
}

fn parse_amount(value: &Value, label: &str) -> Result<Amount, Box<dyn std::error::Error>> {
    let currency = value["currency"]
        .as_str()
        .ok_or_else(|| format!("missing currency in {label}"))?
        .to_string();
    let amount = parse_decimal_value(&value["number"], label)?;
    Ok(Amount { currency, amount })
}

fn parse_quantity(value: &Value) -> Result<Quantity, Box<dyn std::error::Error>> {
    if let Some(_currency) = value.get("currency").and_then(Value::as_str) {
        let amount = parse_decimal_value(&value["number"], "quantity")?;
        return Ok(Quantity {
            amount,
        });
    }

    let positions = value["positions"]
        .as_array()
        .ok_or("missing positions array in quantity")?;
    let first = positions
        .first()
        .ok_or("empty positions array in quantity")?;
    // let currency = first["currency"]
    //     .as_str()
    //     .ok_or("missing currency in quantity position")?
    //     .to_string();
    let amount = parse_decimal_value(&first["number"], "quantity")?;
    Ok(Quantity { amount })
}

fn format_decimal(amount: Decimal) -> String {
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
    if frac_part.chars().all(|c| c == '0') {
        format!("{sign}{with_commas}")
    } else {
        format!("{sign}{with_commas}.{frac_part}")
    }
}

fn format_quantity(quantity: &Quantity) -> String {
    format_decimal(quantity.amount)
}

fn format_amount_cell(amount: Decimal, currency: &str) -> String {
    format!("{} {}", format_decimal(amount), currency)
}

fn print_table(rows: &[LotsRow], exchange: Option<&str>) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    let mut headers = vec![
        Cell::new("Date").set_alignment(CellAlignment::Left),
        Cell::new("Account").set_alignment(CellAlignment::Left),
        Cell::new("Quantity").set_alignment(CellAlignment::Right),
        Cell::new("Symbol").set_alignment(CellAlignment::Left),
        Cell::new("Price").set_alignment(CellAlignment::Right),
        Cell::new("Cost").set_alignment(CellAlignment::Right),
        Cell::new("Value").set_alignment(CellAlignment::Right),
    ];
    if exchange.is_some() {
        headers.push(Cell::new("Converted").set_alignment(CellAlignment::Right));
    }
    table.set_header(headers);

    for row in rows {
        let mut cells = vec![
            Cell::new(&row.date).set_alignment(CellAlignment::Left),
            Cell::new(&row.account).set_alignment(CellAlignment::Left),
            Cell::new(format_quantity(&row.quantity)).set_alignment(CellAlignment::Right),
            Cell::new(&row.symbol).set_alignment(CellAlignment::Left),
            Cell::new(format_amount_cell(row.price, &row.cost.currency))
                .set_alignment(CellAlignment::Right),
            Cell::new(format_amount_cell(row.cost.amount, &row.cost.currency))
                .set_alignment(CellAlignment::Right),
            Cell::new(format_amount_cell(row.value.amount, &row.value.currency))
                .set_alignment(CellAlignment::Right),
        ];
        if let Some(_exchange_currency) = exchange {
            if let Some(ref converted) = row.converted_value {
                cells.push(
                    Cell::new(format_amount_cell(converted.amount, &converted.currency))
                        .set_alignment(CellAlignment::Right),
                );
            } else {
                cells.push(Cell::new("").set_alignment(CellAlignment::Right));
            }
        }
        table.add_row(cells);
    }

    println!("{table}");
}

fn print_average_table(rows: &[AverageLotsRow], exchange: Option<&str>) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    let mut headers = vec![
        Cell::new("Date").set_alignment(CellAlignment::Left),
        Cell::new("Account").set_alignment(CellAlignment::Left),
        Cell::new("Quantity").set_alignment(CellAlignment::Right),
        Cell::new("Symbol").set_alignment(CellAlignment::Left),
        Cell::new("Average Price").set_alignment(CellAlignment::Right),
        Cell::new("Total Cost").set_alignment(CellAlignment::Right),
        Cell::new("Value").set_alignment(CellAlignment::Right),
    ];
    if exchange.is_some() {
        headers.push(Cell::new("Converted").set_alignment(CellAlignment::Right));
    }
    table.set_header(headers);

    for row in rows {
        let mut cells = vec![
            Cell::new(&row.date).set_alignment(CellAlignment::Left),
            Cell::new(&row.account).set_alignment(CellAlignment::Left),
            Cell::new(format_quantity(&row.quantity)).set_alignment(CellAlignment::Right),
            Cell::new(&row.symbol).set_alignment(CellAlignment::Left),
            Cell::new(format_amount_cell(
                row.average_price,
                &row.total_cost.currency,
            ))
            .set_alignment(CellAlignment::Right),
            Cell::new(format_amount_cell(
                row.total_cost.amount,
                &row.total_cost.currency,
            ))
            .set_alignment(CellAlignment::Right),
            Cell::new(format_amount_cell(row.value.amount, &row.value.currency))
                .set_alignment(CellAlignment::Right),
        ];
        if let Some(_exchange_currency) = exchange {
            if let Some(ref converted) = row.converted_value {
                cells.push(
                    Cell::new(format_amount_cell(converted.amount, &converted.currency))
                        .set_alignment(CellAlignment::Right),
                );
            } else {
                cells.push(Cell::new("").set_alignment(CellAlignment::Right));
            }
        }
        table.add_row(cells);
    }

    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_default_active_lots() {
        let opts = LotsOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: None,
            sort: None,
            limit: None,
            no_pager: false,
            sort_by: None,
            average: false,
            active: true,
            show_all: false,
            closed: false,
            ledger: None,
        };

        let query = build_query(&opts);
        assert!(query.contains("cost_number IS NOT NULL"));
        assert!(query
            .contains("GROUP BY account, currency(units(position)), cost_number, cost_currency"));
        assert!(query.contains("HAVING SUM(number(units(position))) > 0"));
        assert!(query.contains("ORDER BY date ASC"));
    }

    #[test]
    fn build_query_with_exchange_lowercase_is_capitalized() {
        // Test for average mode
        let opts = LotsOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: Some("eur".to_string()),  // lowercase
            sort: None,
            limit: None,
            no_pager: false,
            sort_by: None,
            average: true,
            active: true,
            show_all: false,
            closed: false,
            ledger: None,
        };

        let query = build_query(&opts);
        assert!(query.contains("convert(value(SUM(position)), 'EUR')"));
        assert!(!query.contains("convert(value(SUM(position)), 'eur')"));

        // Test for show_all mode
        let opts = LotsOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: Some("usd".to_string()),  // lowercase
            sort: None,
            limit: None,
            no_pager: false,
            sort_by: None,
            average: false,
            active: false,
            show_all: true,
            closed: false,
            ledger: None,
        };

        let query = build_query(&opts);
        assert!(query.contains("convert(value(position), 'USD')"));
        assert!(!query.contains("convert(value(position), 'usd')"));
    }

    #[test]
    fn build_query_with_currency_lowercase_is_capitalized() {
        // Test with single lowercase currency
        let opts = LotsOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec!["eur".to_string()],  // lowercase
            exchange: None,
            sort: None,
            limit: None,
            no_pager: false,
            sort_by: None,
            average: false,
            active: true,
            show_all: false,
            closed: false,
            ledger: None,
        };

        let query = build_query(&opts);
        assert!(query.contains("currency = 'EUR'"));
        assert!(!query.contains("currency = 'eur'"));

        // Test with multiple currencies mixed case
        let opts = LotsOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec!["usd,gbp".to_string()],  // lowercase
            exchange: None,
            sort: None,
            limit: None,
            no_pager: false,
            sort_by: None,
            average: false,
            active: true,
            show_all: false,
            closed: false,
            ledger: None,
        };

        let query = build_query(&opts);
        assert!(query.contains("currency IN ('USD', 'GBP')"));
        assert!(!query.contains("currency IN ('usd', 'gbp')"));
    }

    #[test]
    fn build_query_closed_lots() {
        let opts = LotsOptions {
            account: vec![],
            begin: None,
            end: None,
            date_range: None,
            amount: vec![],
            currency: vec![],
            exchange: None,
            sort: None,
            limit: None,
            no_pager: false,
            sort_by: None,
            average: false,
            active: false,
            show_all: false,
            closed: true,
            ledger: None,
        };

        let query = build_query(&opts);
        assert!(query.contains("cost_number IS NOT NULL"));
        assert!(query
            .contains("GROUP BY account, currency(units(position)), cost_number, cost_currency"));
        assert!(query.contains("HAVING SUM(number(units(position))) <= 0"));
        assert!(query.contains("ORDER BY date ASC"));
    }
}
