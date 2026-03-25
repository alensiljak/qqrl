/// Integration tests for the `reg` / `register` command.
///
/// These tests spawn `qqrl reg` against `tests/sample-ledger.bean` and
/// inspect the printed table for correctness.
use std::path::PathBuf;
use std::process::Command;

fn qqrl_bin() -> PathBuf {
    let mut bin = std::env::current_exe().expect("current_exe");
    bin.pop();
    if bin.ends_with("deps") {
        bin.pop();
    }
    bin.push("qqrl");
    bin
}

fn ledger_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("sample-ledger.bean")
}

/// Run `qqrl reg [args]` and return (stdout, stderr, exit_code)
fn run_reg(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(qqrl_bin())
        .arg("reg")
        .arg("--ledger")
        .arg(ledger_path())
        .args(args)
        .output()
        .expect("failed to run qqrl");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

// ---------------------------------------------------------------------------
// Basic
// ---------------------------------------------------------------------------

#[test]
fn reg_no_args_exits_ok() {
    let (stdout, _stderr, code) = run_reg(&[]);
    assert_eq!(code, 0, "exit code should be 0");
    assert!(stdout.contains("2025-02-01"), "should list Ice Cream date");
    assert!(stdout.contains("Ice Cream Shop"), "should list payee");
    assert!(stdout.contains("Ice Cream"), "should list narration");
    assert!(stdout.contains("Expenses:Sweets"), "should list account");
    assert!(stdout.contains("20.00 EUR"), "should list amount");
}

#[test]
fn reg_shows_expected_columns() {
    let (stdout, _stderr, code) = run_reg(&[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Date"), "should have Date column header");
    assert!(stdout.contains("Account"), "should have Account column header");
    assert!(stdout.contains("Payee"), "should have Payee column header");
    assert!(stdout.contains("Narration"), "should have Narration column header");
    assert!(stdout.contains("Amount"), "should have Amount column header");
    // Running Total should NOT appear without --total
    assert!(!stdout.contains("Running Total"), "Running Total should be absent without -T");
}

// ---------------------------------------------------------------------------
// Account pattern filter
// ---------------------------------------------------------------------------

#[test]
fn reg_filter_by_account() {
    let (stdout, _stderr, code) = run_reg(&["food"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("2025-03-01"));
    assert!(stdout.contains("Grocery Store"));
    assert!(stdout.contains("Groceries"));
    assert!(stdout.contains("Expenses:Food"));
    assert!(stdout.contains("100.00 EUR"));
    assert!(!stdout.contains("Ice Cream"), "Ice Cream should be excluded");
}

#[test]
fn reg_filter_by_payee() {
    let (stdout, _stderr, code) = run_reg(&["@Grocery Store"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Grocery Store"));
    assert!(stdout.contains("Groceries"));
    assert!(!stdout.contains("Ice Cream"), "Ice Cream should be excluded");
}

#[test]
fn reg_filter_excludes_with_not() {
    let (stdout, _stderr, code) = run_reg(&["Assets", "not", "Bank"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets:Cash:"));
    assert!(!stdout.contains("Assets:Bank:"), "Bank accounts should be excluded");
}

#[test]
fn reg_multiple_account_patterns() {
    // Combine payee + account
    let (stdout, _stderr, code) = run_reg(&["@Ice Cream Shop", "-b", "2025-02", "Sweets"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Ice Cream Shop"));
    assert!(stdout.contains("Ice Cream"));
    assert!(stdout.contains("Expenses:Sweets"));
    assert!(!stdout.contains("Grocery Store"), "Grocery Store should be excluded");
}

// ---------------------------------------------------------------------------
// Date filters
// ---------------------------------------------------------------------------

#[test]
fn reg_begin_date_filter() {
    let (stdout, _stderr, code) = run_reg(&["--begin", "2025-04"]);
    assert_eq!(code, 0);
    // Ice Cream is 2025-02-01, should be excluded
    assert!(!stdout.contains("Ice Cream"), "Ice Cream should be excluded");
    // Stock purchase is 2025-04-01, should be included
    assert!(stdout.contains("Buy Stocks"), "Buy Stocks should be included");
}

#[test]
fn reg_end_date_filter() {
    let (stdout, _stderr, code) = run_reg(&["--end", "2025-03-01"]);
    assert_eq!(code, 0);
    // Initial Balance (2025-01-01) and Ice Cream (2025-02-01) should be present
    assert!(stdout.contains("Initial Balance"));
    assert!(stdout.contains("Ice Cream"));
    // Grocery Store (2025-03-01) should be excluded (date < end)
    assert!(!stdout.contains("Groceries"), "Groceries should be excluded");
}

#[test]
fn reg_date_range_month() {
    let (stdout, _stderr, code) = run_reg(&["--date-range", "2025-08"]);
    assert_eq!(code, 0);
    // August 2025 transactions: Transfer to savings (08-01), Holiday Bus (08-15), Holiday Train (08-16)
    assert!(stdout.contains("Transfer to savings"));
    assert!(stdout.contains("Bus"));
    assert!(!stdout.contains("Ice Cream"), "Ice Cream should be excluded");
}

// ---------------------------------------------------------------------------
// Currency filter
// ---------------------------------------------------------------------------

#[test]
fn reg_filter_by_currency() {
    let (stdout, _stderr, code) = run_reg(&["--currency", "EUR"]);
    assert_eq!(code, 0);
    // CHF (3000 CHF) transaction should be excluded
    assert!(!stdout.contains("3,000.00 CHF"), "CHF transaction should be excluded");
    // EUR transactions should be present
    assert!(stdout.contains("EUR"));
}

#[test]
fn reg_filter_by_currency_usd() {
    let (stdout, _stderr, code) = run_reg(&["--currency", "USD"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("7.00 USD"));
    assert!(stdout.contains("Metro"));
    assert!(!stdout.contains("EUR"), "EUR transactions should be excluded");
}

// ---------------------------------------------------------------------------
// Amount filter
// ---------------------------------------------------------------------------

#[test]
fn reg_filter_amount_gt() {
    let (stdout, _stderr, code) = run_reg(&["--amount", ">50"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Grocery Store"));
    assert!(stdout.contains("100.00 EUR"));
    assert!(!stdout.contains("Ice Cream"), "20 EUR should be excluded");
}

#[test]
fn reg_filter_amount_gt_with_currency() {
    let (stdout, _stderr, code) = run_reg(&["--amount", ">50EUR"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Grocery Store"));
    assert!(!stdout.contains("Ice Cream"), "20 EUR should be excluded");
}

// ---------------------------------------------------------------------------
// Running totals (--total / -T)
// ---------------------------------------------------------------------------

#[test]
fn reg_total_flag_shows_running_total() {
    let (stdout, _stderr, code) = run_reg(&["--total"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Running Total"), "Running Total column should appear");
    assert!(stdout.contains("1,000.00 EUR"), "1000 EUR running total should appear");
}

#[test]
fn reg_total_flag_short() {
    let (stdout, _stderr, code) = run_reg(&["-T"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Running Total"));
}

#[test]
fn reg_total_running_sum_accumulates() {
    // Filter to just EUR food/sweets to get predictable running totals
    let (stdout, _stderr, code) = run_reg(&["Expenses:Food", "--total"]);
    assert_eq!(code, 0);
    // Food expenses: 100 EUR (2025-03-01) — single entry so running total = 100.00 EUR
    assert!(stdout.contains("100.00 EUR"));
}

// ---------------------------------------------------------------------------
// Sort
// ---------------------------------------------------------------------------

#[test]
fn reg_sort_by_date_asc() {
    let (stdout, _stderr, code) = run_reg(&["--sort", "date"]);
    assert_eq!(code, 0);
    let ice_cream_pos = stdout.find("Ice Cream").unwrap_or(usize::MAX);
    let grocery_pos = stdout.find("Groceries").unwrap_or(usize::MAX);
    assert!(ice_cream_pos < grocery_pos, "Ice Cream (2025-02) should appear before Groceries (2025-03)");
}

#[test]
fn reg_sort_by_date_desc() {
    let (stdout, _stderr, code) = run_reg(&["--sort", "-date"]);
    assert_eq!(code, 0);
    let ice_cream_pos = stdout.find("Ice Cream").unwrap_or(usize::MAX);
    let grocery_pos = stdout.find("Groceries").unwrap_or(usize::MAX);
    assert!(grocery_pos < ice_cream_pos, "Groceries (2025-03) should appear before Ice Cream (2025-02) in DESC order");
}

// ---------------------------------------------------------------------------
// Limit
// ---------------------------------------------------------------------------

#[test]
fn reg_limit_restricts_rows() {
    let (stdout, _stderr, code) = run_reg(&["--limit", "2"]);
    assert_eq!(code, 0);
    // Count data rows (lines that start with │ but not the header line)
    let data_rows: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains('│') && !l.contains("Date") && !l.contains("Account"))
        .collect();
    assert!(data_rows.len() <= 2, "Should have at most 2 data rows, got {}", data_rows.len());
}
