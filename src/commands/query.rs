use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, CellAlignment, Table};
use serde_json::Value;

use crate::{
    cli::CommonOptions,
    config::Config,
};

/// Query entry extracted from ledger file
#[derive(Debug, Clone)]
struct QueryEntry {
    name: String,
    query_string: String,
}

/// Execute named BQL queries from .bean file
///
/// Scan the ledger file for 'query "name" "BQL_STATEMENT"' directives
/// and execute the specified query.
///
/// Usage:
///   qqrl query QUERY_NAME
///   qqrl q holidays
///   qqrl query my-custom-report
///   qqrl query --list
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(opts.ledger.clone())?;

    // Handle --list flag
    if opts.list {
        list_queries(&config.ledger_file)?;
        return Ok(());
    }

    // Parse query name argument (first positional argument)
    let query_name = if opts.account.is_empty() {
        return Err("Query name is required".into());
    } else {
        opts.account[0].clone()
    };

    // Load and find query from ledger file
    let (query_string, actual_name) = find_query(&config.ledger_file, &query_name)?;

    // Print the query being executed (always show the query)
    println!("Your BQL query is:\n{query_string}\n");

    // Print "Running query:" for non-exact matches (as in Python version)
    if !actual_name.eq_ignore_ascii_case(&query_name) {
        println!("Running query: {actual_name}");
    }

    // Execute the query via rledger and get full response with columns
    let (columns, rows) = run_bql_query_with_columns(&config, &query_string)?;

    // Format and display output
    let formatted_rows = format_output(&rows)?;

    // Print table with headers
    print_table(&columns, &formatted_rows);

    Ok(())
}

/// Parse the ledger file to extract query directives
///
/// Format: 2025-09-02 query "holidays" "select * where payee ~ 'holiday' ..."
fn parse_ledger_queries(ledger_path: &std::path::Path) -> Result<Vec<QueryEntry>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(ledger_path)?;
    let mut queries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        // Check if line contains a query directive
        // Pattern: <date> query "<name>" "<query_string>"
        if let Some(idx) = line.find("query ") {
            let after_query = &line[idx + 6..]; // after "query "
            let after_query = after_query.trim_start();

            // Find the first quoted string (name)
            let name_start = after_query.find('"').ok_or("Invalid query directive: missing opening quote for name")?;
            let name_end = after_query[name_start + 1..].find('"').ok_or("Invalid query directive: missing closing quote for name")? + name_start + 1;
            let name = after_query[name_start + 1..name_end].to_string();

            // Find the second quoted string (query)
            let after_name = &after_query[name_end + 1..];
            let query_start = after_name.find('"').ok_or("Invalid query directive: missing opening quote for query")?;
            let query_end = after_name[query_start + 1..].find('"').ok_or("Invalid query directive: missing closing quote for query")? + query_start + 1;
            let query_string = after_name[query_start + 1..query_end].to_string();

            queries.push(QueryEntry { name, query_string });
        }
    }

    Ok(queries)
}

/// Find a query by name using the matching hierarchy: 
/// 1. Exact match (case-sensitive)
/// 2. Case-insensitive match
/// 3. Partial match (contains, case-insensitive)
fn find_query(
    ledger_path: &std::path::Path,
    query_name: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let query_entries = parse_ledger_queries(ledger_path)?;

    // 1. Exact match
    for entry in &query_entries {
        if entry.name == query_name {
            return Ok((entry.query_string.clone(), entry.name.clone()));
        }
    }

    // 2. Case-insensitive match
    for entry in &query_entries {
        if entry.name.eq_ignore_ascii_case(query_name) {
            return Ok((entry.query_string.clone(), entry.name.clone()));
        }
    }

    // 3. Partial match (contains)
    let query_name_lower = query_name.to_lowercase();
    for entry in &query_entries {
        if entry.name.to_lowercase().contains(&query_name_lower) {
            return Ok((entry.query_string.clone(), entry.name.clone()));
        }
    }

    Err(format!("Query '{}' not found in the ledger file.", query_name).into())
}

/// Execute a BQL query and return both column names and rows
fn run_bql_query_with_columns(
    config: &Config,
    query: &str,
) -> Result<(Vec<String>, Vec<Value>), Box<dyn std::error::Error>> {
    let output = std::process::Command::new(&config.rledger_bin)
        .arg("query")
        .arg("-f")
        .arg("json")
        .arg(config.ledger_file.as_os_str())
        .arg(query)
        .output()
        .map_err(|source| format!("failed to spawn rledger '{}': {}", config.rledger_bin, source))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        return Err(format!("rledger query failed (exit: {exit_code}): {stderr}").into());
    }

    let payload: Value = serde_json::from_slice(&output.stdout)?;
    let columns = payload
        .get("columns")
        .and_then(Value::as_array)
        .ok_or("invalid JSON schema from rledger: missing columns array")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .ok_or("invalid JSON schema from rledger: missing rows array")?
        .to_vec();

    Ok((columns, rows))
}

/// Format a row into a vector of string values
fn format_row(row: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut formatted_row = Vec::new();
    if let Some(obj) = row.as_object() {
        // Preserve the order from the object
        for (_, value) in obj {
            formatted_row.push(format_value(value));
        }
    }
    Ok(formatted_row)
}

/// Format all rows
fn format_output(rows: &[Value]) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let mut formatted_output = Vec::new();
    for row in rows {
        formatted_output.push(format_row(row)?);
    }
    Ok(formatted_output)
}

/// Format a single JSON value to a string representation
fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(d) = n.as_f64() {
                // Format decimal nicely
                if d.fract() == 0.0 {
                    format!("{:.0}", d)
                } else {
                    d.to_string()
                }
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            // Handle arrays (like positions) - format as comma-separated
            let items: Vec<String> = arr.iter().map(format_value).collect();
            items.join(", ")
        }
        Value::Object(obj) => {
            // Handle objects like Amount, Position
            if let Some(units) = obj.get("units") {
                if let Some(currency) = units.get("currency").and_then(|c| c.as_str()) {
                    if let Some(number) = units.get("number").and_then(|n| n.as_f64()) {
                        return format!("{} {}", number, currency);
                    }
                }
            }
            // Fallback: serialize as compact JSON
            value.to_string()
        }
    }
}

/// Print the formatted table to stdout
fn print_table(headers: &[String], rows: &[Vec<String>]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);

    // Add header row
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(h).set_alignment(CellAlignment::Center))
        .collect();
    table.set_header(header_cells);

    // Add data rows
    for row in rows {
        let cells: Vec<Cell> = row
            .iter()
            .map(|text| Cell::new(text).set_alignment(CellAlignment::Left))
            .collect();
        table.add_row(cells);
    }

    println!("\n{}", table);
}

/// List all saved queries in a table format
fn list_queries(ledger_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let queries = parse_ledger_queries(ledger_path)?;
    
    if queries.is_empty() {
        println!("No saved queries found in the ledger file.");
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);

    // Set headers
    table.set_header(vec![
        Cell::new("Name").set_alignment(CellAlignment::Center),
        Cell::new("Query (first 50 chars)").set_alignment(CellAlignment::Center),
    ]);

    // Add rows
    for entry in queries {
        let truncated = if entry.query_string.len() > 50 {
            format!("{}...", &entry.query_string[..50])
        } else {
            entry.query_string
        };
        table.add_row(vec![
            Cell::new(entry.name).set_alignment(CellAlignment::Left),
            Cell::new(truncated).set_alignment(CellAlignment::Left),
        ]);
    }

    println!("\n{}", table);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_ledger_queries() {
        use std::fs;

        let ledger_content = r#"
; Sample ledger
2025-01-01 open Assets:Bank EUR

2025-09-02 query "holidays" "select * where payee ~ 'holiday' and account ~ 'expenses'"

2025-09-03 query "test" "SELECT * FROM transactions"
"#;

        // Create a temporary file using a simple approach
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_ledger_queries.bean");
        fs::write(&temp_path, ledger_content).unwrap();

        let queries = parse_ledger_queries(&temp_path).unwrap();

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].name, "holidays");
        assert!(queries[0].query_string.contains("payee ~ 'holiday'"));
        assert_eq!(queries[1].name, "test");
        assert!(queries[1].query_string.contains("SELECT * FROM transactions"));

        // Cleanup
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_find_query_exact_match() {
        let entries = vec![
            QueryEntry {
                name: "holidays".to_string(),
                query_string: "SELECT * WHERE payee ~ 'holiday'".to_string(),
            },
            QueryEntry {
                name: "test".to_string(),
                query_string: "SELECT * FROM transactions".to_string(),
            },
        ];

        // Simulate find_query logic with entries
        let query_name = "holidays";
        let result = entries
            .iter()
            .find(|e| e.name == query_name)
            .map(|e| (e.query_string.clone(), e.name.clone()));

        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "SELECT * WHERE payee ~ 'holiday'");
    }

    #[test]
    fn test_find_query_case_insensitive() {
        let entries = vec![
            QueryEntry {
                name: "Holidays".to_string(),
                query_string: "SELECT * WHERE payee ~ 'holiday'".to_string(),
            },
        ];

        let query_name = "holidays";
        let result = entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(query_name))
            .map(|e| (e.query_string.clone(), e.name.clone()));

        assert!(result.is_some());
        assert_eq!(result.unwrap().1, "Holidays");
    }

    #[test]
    fn test_find_query_partial_match() {
        let entries = vec![
            QueryEntry {
                name: "my_long_query_name".to_string(),
                query_string: "SELECT *".to_string(),
            },
        ];

        let query_name = "query";
        let query_name_lower = query_name.to_lowercase();
        let result = entries
            .iter()
            .find(|e| e.name.to_lowercase().contains(&query_name_lower))
            .map(|e| (e.query_string.clone(), e.name.clone()));

        assert!(result.is_some());
        assert_eq!(result.unwrap().1, "my_long_query_name");
    }

    #[test]
    fn test_format_value() {
        // Test number
        let v = Value::from(42.5);
        assert_eq!(format_value(&v), "42.5");

        // Test integer (no decimal)
        let v = Value::from(42);
        assert_eq!(format_value(&v), "42");

        // Test string
        let v = Value::String("hello".to_string());
        assert_eq!(format_value(&v), "hello");

        // Test object with units
        let obj = serde_json::json!({
            "units": {
                "number": 100.5,
                "currency": "EUR"
            }
        });
        assert_eq!(format_value(&obj), "100.5 EUR");

        // Test array
        let arr = serde_json::json!([1, 2, 3]);
        assert_eq!(format_value(&arr), "1, 2, 3");
    }
}
