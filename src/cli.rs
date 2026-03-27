use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Quick Query for RustLedger — Rust port of ledger2bql
///
/// A fast CLI that translates Ledger CLI syntax to BQL queries,
/// executes them via rledger, and formats the output.
#[derive(Debug, Parser)]
#[command(name = "qqrl")]
#[command(about, long_about = None)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Account balances
    #[command(visible_alias = "b", visible_alias = "bal")]
    Balance(CommonOptions),

    /// Transaction register
    #[command(visible_alias = "r", visible_alias = "reg")]
    Register(CommonOptions),

    /// Execute named queries from .bean file
    #[command(visible_alias = "q")]
    Query(CommonOptions),

    /// Investment lots and cost basis
    #[command(visible_alias = "l", visible_alias = "lot")]
    Lots(LotsOptions),

    /// Balance assertions
    #[command(visible_alias = "a")]
    Assert(CommonOptions),

    /// Price history
    #[command(visible_alias = "p")]
    Price(CommonOptions),
}

/// Options shared by all commands (except query)
#[derive(Debug, Parser)]
pub struct CommonOptions {
    /// Account pattern(s) to filter (supports multiple: 'Assets not Bank @Employer')
    #[arg(value_name = "PATTERN", num_args = 0..)]
    pub account: Vec<String>,

    /// Start date (YYYY-MM-DD or date range format)
    #[arg(short, long)]
    pub begin: Option<String>,

    /// End date (YYYY-MM-DD or date range format)
    #[arg(short, long)]
    pub end: Option<String>,

    /// Date range (format: YYYY-MM..YYYY-MM or YYYY-MM-DD..YYYY-MM-DD)
    #[arg(short, long)]
    pub date_range: Option<String>,

    /// Amount filter(s) — e.g., '>100EUR', '<=50USD'
    /// Can be repeated: -a '>10' -a '<200'
    #[arg(short, long)]
    pub amount: Vec<String>,

    /// Currency filter(s) — e.g., 'EUR' or 'EUR,USD'
    /// Can be repeated: -c EUR -c USD
    #[arg(short, long)]
    pub currency: Vec<String>,

    /// Exchange currency — convert all amounts to this currency
    #[arg(short = 'X', long)]
    pub exchange: Option<String>,

    /// Sort by field(s) — prefix with '-' for descending
    /// e.g., 'account', '-amount', 'date account'
    #[arg(short = 'S', long, allow_hyphen_values = true)]
    pub sort: Option<String>,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Show running total / summary
    #[arg(short = 'T', long)]
    pub total: bool,

    /// Disable pager output
    #[arg(long)]
    pub no_pager: bool,

    /// Show account hierarchy (balance command only)
    #[arg(short = 'H', long)]
    pub hierarchy: bool,

    /// Include empty accounts (balance command only)
    #[arg(long)]
    pub empty: bool,

    /// Limit account tree depth (balance command only)
    #[arg(short = 'D', long)]
    pub depth: Option<u32>,

    /// Exclude accounts with zero balance (balance command only)
    #[arg(short = 'Z', long)]
    pub zero: bool,

    /// Ledger file path (overrides LEDGER_FILE env var)
    #[arg(long)]
    pub ledger: Option<PathBuf>,
}

/// Options for the lots command.
#[derive(Debug, Parser)]
pub struct LotsOptions {
    /// Account pattern(s) to filter (supports multiple: 'Assets not Bank @Employer')
    #[arg(value_name = "PATTERN", num_args = 0..)]
    pub account: Vec<String>,

    /// Start date (YYYY-MM-DD or date range format)
    #[arg(short, long)]
    pub begin: Option<String>,

    /// End date (YYYY-MM-DD or date range format)
    #[arg(short, long)]
    pub end: Option<String>,

    /// Date range (format: YYYY-MM..YYYY-MM or YYYY-MM-DD..YYYY-MM-DD)
    #[arg(short, long)]
    pub date_range: Option<String>,

    /// Amount filter(s) — e.g., '>100EUR', '<=50USD'
    #[arg(short, long)]
    pub amount: Vec<String>,

    /// Currency filter(s) — e.g., 'EUR' or 'EUR,USD'
    #[arg(short, long)]
    pub currency: Vec<String>,

    /// Exchange currency — convert all amounts to this currency
    #[arg(short = 'X', long)]
    pub exchange: Option<String>,

    /// Sort by field(s) — prefix with '-' for descending
    #[arg(short = 'S', long, allow_hyphen_values = true)]
    pub sort: Option<String>,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Disable pager output
    #[arg(long)]
    pub no_pager: bool,

    /// Sort lots by date, price, or symbol
    #[arg(short = 's', long, value_parser = ["date", "price", "symbol"])]
    pub sort_by: Option<String>,

    /// Show average cost for each symbol
    #[arg(short = 'A', long)]
    pub average: bool,

    /// Show only active/open lots
    #[arg(long, default_value_t = true, overrides_with = "show_all", overrides_with = "closed")]
    pub active: bool,

    /// Show all lots, including sold ones
    #[arg(long = "all", overrides_with = "active", overrides_with = "closed")]
    pub show_all: bool,

    /// Show only closed/inactive lots
    #[arg(long, overrides_with = "active", overrides_with = "show_all")]
    pub closed: bool,

    /// Ledger file path (overrides LEDGER_FILE env var)
    #[arg(long)]
    pub ledger: Option<PathBuf>,
}
