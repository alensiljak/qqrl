use crate::cli::CommonOptions;

/// Account balances command
///
/// Query and display account balances at a point in time or over a period.
///
/// Usage:
///   qqrl balance [PATTERN] [OPTIONS]
///   qqrl bal Assets --begin 2025-01-01 --end 2025-12-31
///   qqrl balance --hierarchy     # Show account tree
///   qqrl balance --currency EUR  # Filter by currency
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("bal command: not yet implemented");
    eprintln!("Options: {:?}", opts);

    // TODO: Implement balance command
    // 1. Load config
    // 2. Build BQL query from options
    // 3. Execute via rledger
    // 4. Parse JSON response
    // 5. Format output (with hierarchy if requested)
    // 6. Display via pager or stdout

    Ok(())
}
