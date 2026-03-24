use crate::cli::CommonOptions;

/// Transaction register command
///
/// Display transactions matching criteria with running totals per currency.
///
/// Usage:
///   qqrl register [PATTERN] [OPTIONS]
///   qqrl reg Assets:Bank --begin 2025-06-01 --total
///   qqrl register --currency EUR --exchange CHF
///   qqrl reg --limit 20
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("reg command: not yet implemented");
    eprintln!("Options: {:?}", opts);

    // TODO: Implement register command
    // 1. Load config
    // 2. Build BQL query from options
    // 3. Execute via rledger
    // 4. Parse JSON response
    // 5. Format output with running totals per currency
    // 6. Apply --exchange currency conversion if specified
    // 7. Display via pager or stdout

    Ok(())
}
