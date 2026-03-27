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

fn run_cmd(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(qqrl_bin())
        .arg("query")
        .arg("--ledger")
        .arg(ledger_path())
        .args(args)
        .output()
        .expect("failed to run qqrl query");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn query_exact_name() {
    let (stdout, stderr, code) = run_cmd(&["holidays"]);

    assert_eq!(code, 0, "query should succeed: {stderr}");
    assert!(stdout.contains("Your BQL query is:"));
    assert!(stdout.contains("select * where payee ~ 'holiday'"));
    // rledger returns dates in the output
    assert!(stdout.contains("2025-08-15"));
    // Should not display "Running query:" when exact match
    assert!(!stdout.contains("Running query:"));
}

#[test]
fn query_partial_match() {
    let (stdout, stderr, code) = run_cmd(&["holi"]);

    assert_eq!(code, 0, "query should succeed: {stderr}");
    assert!(stdout.contains("Your BQL query is:"));
    assert!(stdout.contains("select * where payee ~ 'holiday'"));
    assert!(stdout.contains("2025-08-15"));
    // Should display "Running query:" when partial match
    assert!(stdout.contains("Running query: holidays"));
}

#[test]
fn query_nonexistent() {
    let (stdout, stderr, code) = run_cmd(&["nonexistent"]);

    assert_ne!(code, 0, "query should fail for nonexistent query");
    let combined = format!("{stdout} {stderr}");
    assert!(
        combined.contains("not found"),
        "error message should indicate query not found, got: {combined}"
    );
}

#[test]
fn query_missing_argument() {
    let (stdout, stderr, code) = run_cmd(&[]);

    assert_ne!(code, 0, "query should fail when no query name provided");
    let combined = format!("{stdout} {stderr}");
    assert!(
        combined.contains("required") || combined.contains("missing") || combined.contains("Query name"),
        "error should indicate missing argument, got: {combined}"
    );
}
