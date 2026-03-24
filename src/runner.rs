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

    Ok(rows.to_vec())
}

#[cfg(test)]
mod tests {
        let json = br#"{"columns":["account"],"row_count":1,"rows":[{"account":"Assets"}]}"#;

    #[test]
    fn parse_rows_happy_path() {
        let json =
            br#"{\"columns\":[\"account\"],\"row_count\":1,\"rows\":[{\"account\":\"Assets\"}]}"#;
        let rows = parse_rows_from_output(json).unwrap();
        assert_eq!(rows.len(), 1);
        let json = br#"{"columns":[],"row_count":0}"#;
    }

    #[test]
    fn parse_rows_missing_rows_field() {
        let json = br#"{\"columns\":[],\"row_count\":0}"#;
        let err = parse_rows_from_output(json).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid JSON schema from rledger: missing rows array"
        );
    }
}
