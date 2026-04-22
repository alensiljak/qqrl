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

fn zero_net_ledger_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("zero-net-position.bean")
}

fn run_lots(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(qqrl_bin())
        .arg("lots")
        .arg("--ledger")
        .arg(ledger_path())
        .args(args)
        .output()
        .expect("failed to run qqrl lots");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn lots_default_shows_active_open_lots() {
    let (stdout, stderr, code) = run_lots(&[]);
    assert_eq!(code, 0, "lots should succeed: {stderr}");
    assert!(stdout.contains("Date"));
    assert!(stdout.contains("Quantity"));
    assert!(stdout.contains("Price"));
    assert!(stdout.contains("2025-09-10"));
    assert!(stdout.contains("Equity:Stocks"));
    assert!(stdout.contains("ABC"));
    assert!(stdout.contains("1.30 EUR"));
    assert!(stdout.contains("5.20 EUR"));
    assert!(
        !stdout.contains("2025-04-01"),
        "closed lot should not be shown"
    );
}

#[test]
fn lots_all_shows_buys_and_sells() {
    let (stdout, stderr, code) = run_lots(&["--all"]);
    assert_eq!(code, 0, "lots --all should succeed: {stderr}");
    assert!(stdout.contains("2025-04-01"));
    assert!(stdout.contains("2025-04-02"));
    assert!(stdout.contains("2025-09-09"));
    assert!(stdout.contains("2025-09-10"));
    assert!(stdout.contains("-5"));
    assert!(stdout.contains("-3"));
}

#[test]
fn lots_average_shows_average_price_and_total_cost() {
    let (stdout, stderr, code) = run_lots(&["--average"]);
    assert_eq!(code, 0, "lots --average should succeed: {stderr}");
    assert!(stdout.contains("Average Price"));
    assert!(stdout.contains("Total Cost"));
    assert!(stdout.contains("ABC"));
    assert!(stdout.contains("1.30 EUR"));
    assert!(stdout.contains("5.20 EUR"));
}

#[test]
fn lots_filters_by_account() {
    let (stdout, stderr, code) = run_lots(&["Stocks"]);
    assert_eq!(code, 0, "lots account filter should succeed: {stderr}");
    assert!(stdout.contains("Equity:Stocks"));
}

#[test]
fn lots_filters_by_date_range() {
    let (stdout, stderr, code) = run_lots(&["--date-range", "2025-04"]);
    assert_eq!(code, 0, "lots date range should succeed: {stderr}");
    assert!(stdout.contains("2025-04-02") || stdout.contains("2025-04-01"));
    assert!(!stdout.contains("2025-09-10"));
}

#[test]
fn lots_filters_by_currency() {
    let (stdout, stderr, code) = run_lots(&["--currency", "ABC"]);
    assert_eq!(code, 0, "lots currency filter should succeed: {stderr}");
    assert!(stdout.contains("ABC"));
}

#[test]
fn lots_filters_by_amount() {
    let (stdout, stderr, code) = run_lots(&["--all", "--amount", ">0"]);
    assert_eq!(code, 0, "lots amount filter should succeed: {stderr}");
    assert!(stdout.contains("2025-04-01"));
    assert!(stdout.contains("2025-04-02"));
    assert!(!stdout.contains("-5"));
    assert!(!stdout.contains("-3"));
}

#[test]
fn lots_sort_by_price_desc() {
    let (stdout, stderr, code) = run_lots(&["--all", "--sort", "-price"]);
    assert_eq!(code, 0, "lots sort should succeed: {stderr}");

    let pos_130 = stdout.find("1.30 EUR").unwrap_or(usize::MAX);
    let pos_125 = stdout.find("1.25 EUR").unwrap_or(usize::MAX);
    assert!(pos_130 < pos_125, "price 1.30 should sort before 1.25");
}

#[test]
fn lots_limit_restricts_rows() {
    let (stdout, stderr, code) = run_lots(&["--all", "--limit", "2"]);
    assert_eq!(code, 0, "lots limit should succeed: {stderr}");

    let data_rows: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("│") && line.contains("Equity:Stocks"))
        .collect();
    assert_eq!(data_rows.len(), 2, "expected exactly 2 data rows: {stdout}");
}

#[test]
fn lots_average_skips_zero_net_quantity() {
    // Regression: --average used to panic when a group's net quantity was 0
    // (all lots sold in that account), because the BQL query divided by zero.
    // Alpha has net 0 (all sold); Beta still holds 5 XYZ.
    let output = Command::new(qqrl_bin())
        .arg("lots")
        .arg("--ledger")
        .arg(zero_net_ledger_path())
        .arg("--average")
        .output()
        .expect("failed to run qqrl lots --average");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    assert_eq!(code, 0, "lots --average must not crash on zero-net groups: {stderr}");
    assert!(!stdout.contains("Alpha"), "zero-net Alpha account must be excluded: {stdout}");
    assert!(stdout.contains("Beta"), "positive-net Beta account must appear: {stdout}");
    assert!(stdout.contains("7 USD"), "Beta average price must be shown: {stdout}");
}
