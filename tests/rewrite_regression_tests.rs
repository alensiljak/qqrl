/// Regression tests for gaps identified during the ledger2bql -> qqrl rewrite.
///
/// These tests are split into two groups:
/// - current guardrail behavior that should remain stable until the gap is fixed
/// - ignored future-behavior tests that document the intended behavior once the
///   underlying rledger support is available
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

fn run_rledger_query(query: &str) -> (String, String, i32) {
    let output = Command::new("rledger")
        .arg("query")
        .arg(ledger_path())
        .arg(query)
        .output()
        .expect("failed to run rledger");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

fn run_cmd(subcommand: &str, args: &[&str]) -> (String, String, i32) {
    let output = Command::new(qqrl_bin())
        .arg(subcommand)
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

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[test]
fn rledger_supports_sum_of_converted_positions() {
    let (stdout, stderr, code) = run_rledger_query(
        "SELECT account, sum(convert(position, 'EUR')) WHERE account = 'Assets:Bank:Bank03581' GROUP BY account",
    );

    assert_eq!(code, 0, "sum(convert(position, ...)) should work: {stderr}");
    assert!(stdout.contains("Assets:Bank:Bank03581"));
    assert!(
        contains_any(&stdout, &["3,194.1000 EUR", "3194.1000 EUR"]),
        "converted result should be returned in EUR, got: {stdout}"
    );
}

#[test]
fn rledger_supports_convert_of_summed_positions() {
    let (stdout, stderr, code) = run_rledger_query(
        "SELECT account, convert(sum(position), 'EUR') WHERE account = 'Assets:Bank:Bank03581' GROUP BY account",
    );

    assert_eq!(code, 0, "convert(sum(position), ...) should now work: {stderr}");
    assert!(stdout.contains("Assets:Bank:Bank03581"));
    assert!(
        contains_any(&stdout, &["3,194.1000 EUR", "3194.1000 EUR"]),
        "converted result should be returned in EUR, got: {stdout}"
    );
}

#[test]
fn bal_exchange_converts_chf_balance_to_eur() {
    let (stdout, stderr, code) = run_cmd("bal", &["--exchange", "EUR", "Assets:Bank:Bank03581"]);

    assert_eq!(
        code, 0,
        "bal --exchange should succeed once supported: {stderr}"
    );
    assert!(stdout.contains("Assets:Bank:Bank03581"));
    assert!(
        contains_any(&stdout, &["3,000.00 CHF", "3000.00 CHF", "3000 CHF"]),
        "original balance should still be shown, got: {stdout}"
    );
    assert!(
        stdout.contains("Total (EUR)") || stdout.contains("Converted"),
        "exchange output should include a converted column, got: {stdout}"
    );
    assert!(
        contains_any(&stdout, &["3,194.10 EUR", "3194.10 EUR", "3194.1 EUR"]),
        "converted CHF balance should be shown in EUR, got: {stdout}"
    );
}

#[test]
fn reg_exchange_converts_chf_posting_to_eur() {
    let (stdout, stderr, code) = run_cmd("reg", &["--exchange", "EUR", "Assets:Bank:Bank03581"]);

    assert_eq!(
        code, 0,
        "reg --exchange should succeed once supported: {stderr}"
    );
    assert!(stdout.contains("2025-07-15"));
    assert!(stdout.contains("Assets:Bank:Bank03581"));
    assert!(
        stdout.contains("Amount (EUR)") || stdout.contains("Converted"),
        "exchange output should include a converted amount column, got: {stdout}"
    );
    assert!(
        contains_any(&stdout, &["3,194.10 EUR", "3194.10 EUR", "3194.1 EUR"]),
        "converted CHF posting should be shown in EUR, got: {stdout}"
    );
}

#[test]
fn reg_exchange_total_shows_converted_running_total() {
    let (stdout, stderr, code) = run_cmd(
        "reg",
        &["--exchange", "EUR", "--total", "Assets:Bank:Bank03581"],
    );

    assert_eq!(code, 0, "reg --exchange --total should succeed: {stderr}");
    assert!(
        stdout.contains("Running Total"),
        "original running total column should remain"
    );
    assert!(
        stdout.contains("Total (EUR)") || stdout.contains("Converted Total"),
        "converted running total column should appear, got: {stdout}"
    );
    assert!(
        contains_any(&stdout, &["3,194.10 EUR", "3194.10 EUR", "3194.1 EUR"]),
        "converted running total should be shown in EUR, got: {stdout}"
    );
}
