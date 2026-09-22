use std::process::Command;

use serde_json::Value;

use crate::config::Config;

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("failed to spawn rledger '{bin}': {source}")]
    SpawnError {
        bin: String,
        #[source]
        source: std::io::Error,
    },

    #[error("rledger query failed (exit: {exit_code}): {stderr}")]
    QueryFailed { exit_code: i32, stderr: String },

    #[error("invalid JSON from rledger: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("invalid JSON schema from rledger: missing rows array")]
    MissingRows,
}

/// Execute a BQL query via external rledger and return JSON rows.
pub fn run_bql_query(config: &Config, query: &str) -> Result<Vec<Value>, RunnerError> {
    let output = Command::new(&config.rledger_bin)
        .arg("query")
        .arg("-f")
        .arg("json")
        .arg(config.ledger_file.as_os_str())
        .arg(query)
        .output()
        .map_err(|source| RunnerError::SpawnError {
            bin: config.rledger_bin.clone(),
            source,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        return Err(RunnerError::QueryFailed { exit_code, stderr });
    }

    parse_rows_from_output(&output.stdout)
}

fn parse_rows_from_output(stdout: &[u8]) -> Result<Vec<Value>, RunnerError> {
    let payload: Value = serde_json::from_slice(stdout)?;
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .ok_or(RunnerError::MissingRows)?;

    let columns: Vec<String> = payload
        .get("columns")
        .and_then(Value::as_array)
        .map(|cols| {
            cols.iter()
                .filter_map(|c| c.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(rows.iter().map(|row| normalize_row(row, &columns)).collect())
}

/// rledger emits each row as a positional array ordered per `columns`,
/// e.g. `["Assets:Bank", {"positions": [...]}]`, rather than as a JSON
/// object keyed by column name. Zip it into an object so callers can
/// index rows by field name (e.g. `row["account"]`).
fn normalize_row(row: &Value, columns: &[String]) -> Value {
    let Some(values) = row.as_array() else {
        return row.clone();
    };

    let map = columns
        .iter()
        .zip(values.iter())
        .map(|(col, val)| (col.clone(), val.clone()))
        .collect();

    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::parse_rows_from_output;

    #[test]
    fn parse_rows_happy_path() {
        let json = br#"{"columns":["account"],"row_count":1,"rows":[{"account":"Assets"}]}"#;
        let rows = parse_rows_from_output(json).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["account"], "Assets");
    }

    #[test]
    fn parse_rows_missing_rows_field() {
        let json = br#"{"columns":[],"row_count":0}"#;
        let err = parse_rows_from_output(json).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid JSON schema from rledger: missing rows array"
        );
    }

    /// rledger actually emits each row as a positional array (ordered per
    /// "columns"), not as a JSON object keyed by column name. Callers
    /// (balance/assert/register/lots/price) all index rows by field name,
    /// e.g. `row["account"]`, so rows must be normalized into objects here.
    #[test]
    fn parse_rows_normalizes_array_shaped_rows_into_objects() {
        let json = br#"{
            "columns": ["account", "balance"],
            "row_count": 1,
            "rows": [
                ["Assets:Bank:Checking", {"positions": [{"currency": "EUR", "number": "1369.80"}]}]
            ]
        }"#;
        let rows = parse_rows_from_output(json).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["account"], "Assets:Bank:Checking");
        assert_eq!(rows[0]["balance"]["positions"][0]["currency"], "EUR");
    }
}
