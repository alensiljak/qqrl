use crate::cli::CommonOptions;

/// Investment lots and cost basis
///
/// Display inventory positions with cost basis, lot dates, and current values.
/// Useful for tracking stock purchases, gains/losses, and tax reporting.
///
/// Usage:
///   qqrl lots [PATTERN] [OPTIONS]
///   qqrl l Equity:Stocks
///   qqrl lots --exchange EUR   # Convert lot values to EUR
///   qqrl lots --total
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("lots command: not yet implemented");
    eprintln!("Options: {:?}", opts);

    // TODO: Implement lots command
    // 1. Load config
    // 2. Build BQL query from options (uses cost_number, value(SUM(position)))
    // 3. Execute via rledger
    // 4. Parse JSON response — handle Inventory/Position arithmetic
    // 5. Format output with cost basis, lot dates, current value
    // 6. Calculate gains/losses if needed
    // 7. Apply --exchange currency conversion
    // 8. Display via pager or stdout

    Ok(())
}
