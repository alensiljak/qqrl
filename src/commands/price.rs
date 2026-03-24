use crate::cli::CommonOptions;

/// Price history
///
/// Display price history from the ledger file.
///
/// ⚠️  BLOCKED: Requires rledger #prices system table support.
/// Feature request: https://github.com/rustledger/rustledger/issues/...
///
/// Usage:
///   qqrl price [COMMODITY] [OPTIONS]
///   qqrl p EUR
///   qqrl price --begin 2025-01-01 USD
pub fn run(_opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    return Err("The 'price' command is not yet available.\n\
         It requires rledger to support the #prices system table.\n\
         \n\
         Pending feature request: https://github.com/rustledger/rustledger/issues/..."
        .into());
}
