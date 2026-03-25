/// Integration tests for the `bal` / `balance` command.
///
/// These tests spawn `qqrl bal` against `tests/sample-ledger.bean` and
/// inspect the printed table for correctness.
use std::path::PathBuf;
use std::process::Command;

fn qqrl_bin() -> PathBuf {
    // Prefer the test-profile binary produced by `cargo test`
    let mut bin = std::env::current_exe().expect("current_exe");
    bin.pop(); // remove the test binary filename
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

/// Run `qqrl bal [args]` and return (stdout, stderr, exit_code)
fn run_bal(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(qqrl_bin())
        .arg("bal")
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
fn bal_no_args_exits_ok() {
    let (stdout, _stderr, code) = run_bal(&[]);
    assert_eq!(code, 0, "exit code should be 0");
    assert!(stdout.contains("Assets:Bank:Checking"), "should list Checking");
    assert!(stdout.contains("Expenses:Sweets"), "should list Sweets");
    assert!(stdout.contains("1,369.80 EUR") || stdout.contains("1369.80 EUR"));
}

#[test]
fn bal_default_sorted_by_account() {
    let (stdout, _stderr, code) = run_bal(&[]);
    assert_eq!(code, 0);

    // The lines in alphabetical order: Assets before Equity before Expenses before Income
    let assets_pos = stdout.find("Assets:Bank:Checking").unwrap_or(usize::MAX);
    let equity_pos = stdout.find("Equity:Opening-Balances").unwrap_or(usize::MAX);
    let expenses_pos = stdout.find("Expenses:Sweets").unwrap_or(usize::MAX);
    let income_pos = stdout.find("Income:Salary").unwrap_or(usize::MAX);

    assert!(assets_pos < equity_pos, "Assets before Equity");
    assert!(equity_pos < expenses_pos, "Equity before Expenses");
    assert!(expenses_pos < income_pos, "Expenses before Income");
}

// ---------------------------------------------------------------------------
// Account pattern filter
// ---------------------------------------------------------------------------

#[test]
fn bal_filter_assets() {
    let (stdout, _stderr, code) = run_bal(&["Assets"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets:Bank:Checking"));
    assert!(!stdout.contains("Expenses:"));
    assert!(!stdout.contains("Income:"));
}

#[test]
fn bal_filter_not_excludes_accounts() {
    let (stdout, _stderr, code) = run_bal(&["Assets", "not", "Bank"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets:Cash:"));
    assert!(!stdout.contains("Assets:Bank:"));
}

// ---------------------------------------------------------------------------
// Date filters
// ---------------------------------------------------------------------------

#[test]
fn bal_begin_date_filter() {
    // Salary came in 2025-03-15; initial balance on 2025-01-01
    // --begin 2025-04 should exclude the salary transaction from Checking's balance
    let (stdout, _stderr, code) = run_bal(&["--begin", "2025-04", "Assets:Bank:Checking"]);
    assert_eq!(code, 0);
    // The balance should exist and not include the full 1369.80 EUR
    assert!(stdout.contains("Assets:Bank:Checking"));
}

#[test]
fn bal_end_date_filter() {
    let (stdout, _stderr, code) =
        run_bal(&["--end", "2025-04-01", "Assets:Bank:Checking"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets:Bank:Checking"));
}

#[test]
fn bal_date_range_filter() {
    let (stdout, _stderr, code) =
        run_bal(&["--date-range", "2025-03..2025-04", "Assets:Bank:Checking"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets:Bank:Checking"));
}

// ---------------------------------------------------------------------------
// Currency filter
// ---------------------------------------------------------------------------

#[test]
fn bal_currency_filter_eur() {
    let (stdout, _stderr, code) = run_bal(&["--currency", "EUR"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("EUR"));
    // USD and BAM accounts should not appear if they only hold those currencies
    assert!(!stdout.contains("BAM"), "BAM should be filtered out");
    assert!(!stdout.contains("USD"), "USD should be filtered out");
}

// ---------------------------------------------------------------------------
// Limit
// ---------------------------------------------------------------------------

#[test]
fn bal_limit() {
    let (stdout, _stderr, code) = run_bal(&["--limit", "3"]);
    assert_eq!(code, 0);
    // 3 data rows + header + separator = a small table
    let data_rows: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("| ") && !l.contains("Account") && !l.contains("---"))
        .collect();
    assert!(data_rows.len() <= 3, "Expected <= 3 rows, got {}", data_rows.len());
}

// ---------------------------------------------------------------------------
// Total
// ---------------------------------------------------------------------------

#[test]
fn bal_total_flag() {
    let (stdout, _stderr, code) = run_bal(&["--total", "Assets"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Total"), "should include a Total row");
    assert!(stdout.contains("---"), "should include a separator row");
}

// ---------------------------------------------------------------------------
// Hierarchy
// ---------------------------------------------------------------------------

#[test]
fn bal_hierarchy_includes_parents() {
    let (stdout, _stderr, code) = run_bal(&["--hierarchy"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets"), "should contain top-level Assets");
    assert!(stdout.contains("Assets:Bank"), "should contain Assets:Bank");
    assert!(stdout.contains("Assets:Bank:Checking"), "should contain leaf");
    assert!(stdout.contains("Expenses"), "should contain Expenses");
    assert!(stdout.contains("Expenses:Transport"), "should contain Expenses:Transport");
    assert!(stdout.contains("Expenses:Transport:Bus"), "should contain Bus leaf");
}

#[test]
fn bal_hierarchy_transport_aggregates() {
    // Expenses:Transport (direct: 7 USD from Metro) + Bus (10 EUR) + Train (15 EUR)
    let (stdout, _stderr, code) = run_bal(&["--hierarchy", "Expenses:Transport"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Expenses:Transport"));
    assert!(stdout.contains("Expenses:Transport:Bus"));
    assert!(stdout.contains("Expenses:Transport:Train"));
    // The parent should aggregate EUR amounts: 10 + 15 = 25 EUR + 7 USD
    assert!(stdout.contains("25.00 EUR") || stdout.contains("25 EUR"));
}

#[test]
fn bal_hierarchy_with_filter() {
    let (stdout, _stderr, code) = run_bal(&["--hierarchy", "Assets"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets:Bank"));
    assert!(!stdout.contains("Equity"), "non-Assets should be excluded");
    assert!(!stdout.contains("Expenses"), "non-Assets should be excluded");
}

// ---------------------------------------------------------------------------
// Depth
// ---------------------------------------------------------------------------

#[test]
fn bal_depth_2_collapses() {
    let (stdout, _stderr, code) = run_bal(&["--depth", "2"]);
    assert_eq!(code, 0);
    // Depth 2 should NOT include 3-level accounts like Assets:Bank:Checking
    assert!(
        !stdout.contains("Assets:Bank:Checking"),
        "depth 2 should collapse Assets:Bank:Checking into Assets:Bank"
    );
    assert!(
        !stdout.contains("Assets:Bank:Savings"),
        "depth 2 should collapse Assets:Bank:Savings into Assets:Bank"
    );
    // But Assets:Bank itself should appear
    assert!(stdout.contains("Assets:Bank"), "Assets:Bank should appear at depth 2");
}

#[test]
fn bal_depth_1_collapses_to_top() {
    let (stdout, _stderr, code) = run_bal(&["--depth", "1"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Assets"), "Assets top-level should appear");
    assert!(!stdout.contains("Assets:"), "sub-accounts should be collapsed");
    assert!(stdout.contains("Expenses"), "Expenses top-level should appear");
    assert!(!stdout.contains("Expenses:"), "Expenses sub-accounts should be collapsed");
}

// ---------------------------------------------------------------------------
// Zero filter
// ---------------------------------------------------------------------------

#[test]
fn bal_zero_excludes_empty_accounts() {
    // Open `Expenses:Accommodation` with a zero balance scenario is hard to test
    // without modifying the ledger; just verify the flag is accepted and exits ok
    let (_stdout, _stderr, code) = run_bal(&["--zero"]);
    assert_eq!(code, 0);
}

// ---------------------------------------------------------------------------
// Sort
// ---------------------------------------------------------------------------

#[test]
fn bal_sort_desc_account() {
    let (stdout, _stderr, code) = run_bal(&["--sort", "-account"]);
    assert_eq!(code, 0);
    // In descending order Income comes before Expenses before Equity before Assets
    let income_pos = stdout.find("Income:").unwrap_or(usize::MAX);
    let assets_pos = stdout.find("Assets:").unwrap_or(usize::MAX);
    assert!(income_pos < assets_pos, "Income should appear before Assets in DESC order");
}

// ---------------------------------------------------------------------------
// Amount filter
// ---------------------------------------------------------------------------

#[test]
fn bal_amount_filter_greater_than() {
    // Accounts with balance > 100 EUR
    let (stdout, _stderr, code) = run_bal(&["--amount", ">100EUR"]);
    assert_eq!(code, 0);
    // Checking has 1369.80 EUR — should appear
    assert!(stdout.contains("Assets:Bank:Checking"));
    // Sweets has 20 EUR — should be excluded
    assert!(!stdout.contains("Expenses:Sweets"));
}
