use crate::cli::CommonOptions;

/// Execute named BQL queries from .bean file
///
/// Scan the ledger file for 'query "name" "BQL_STATEMENT"' directives
/// and execute the specified query.
///
/// Usage:
///   qqrl query QUERY_NAME
///   qqrl q holidays
///   qqrl query my-custom-report
pub fn run(opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("query command: not yet implemented");
    eprintln!("Options: {:?}", opts);

    // TODO: Implement query command
    // 1. Load config
    // 2. Scan ledger file for 'query "name" "STATEMENT"' directives
    // 3. Find the requested query by name
    // 4. Execute the BQL statement via rledger
    // 5. Parse JSON response
    // 6. Format output
    // 7. Display via pager or stdout

    Ok(())
}
