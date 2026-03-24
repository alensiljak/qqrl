use std::env;
use std::path::PathBuf;

/// Configuration for qqrl
#[derive(Debug)]
pub struct Config {
    pub ledger_file: PathBuf,
    pub rledger_bin: String,
}

impl Config {
    /// Load configuration from environment and .env file
    pub fn load(ledger_override: Option<PathBuf>) -> Result<Self, ConfigError> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        // Resolve ledger file path
        let ledger_file = if let Some(path) = ledger_override {
            path
        } else if let Ok(path) = env::var("LEDGER_FILE") {
            PathBuf::from(path)
        } else if let Ok(path) = env::var("BEANCOUNT_FILE") {
            PathBuf::from(path)
        } else {
            return Err(ConfigError::MissingLedgerFile);
        };

        // Verify ledger file exists
        if !ledger_file.exists() {
            return Err(ConfigError::LedgerFileNotFound(ledger_file));
        }

        // Resolve rledger binary path
        let rledger_bin = env::var("RLEDGER_BIN").unwrap_or_else(|_| "rledger".to_string());

        Ok(Config {
            ledger_file,
            rledger_bin,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("LEDGER_FILE or BEANCOUNT_FILE environment variable not set")]
    MissingLedgerFile,

    #[error("Ledger file not found: {0}")]
    LedgerFileNotFound(PathBuf),
}
