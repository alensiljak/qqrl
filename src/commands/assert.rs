use crate::cli::CommonOptions;

/// Balance assertions
///
/// Display balance assertions from the ledger file and verify them against
/// rledger's computed balances.
///
/// ⚠️  BLOCKED: Requires rledger #balances system table support.
/// Feature request: https://github.com/rustledger/rustledger/issues/...
///
/// Usage:
///   qqrl assert [PATTERN] [OPTIONS]
///   qqrl a Assets
///   qqrl assert --date 2025-12-31
pub fn run(_opts: CommonOptions) -> Result<(), Box<dyn std::error::Error>> {
    return Err("The 'assert' command is not yet available.\n\
         It requires rledger to support the #balances system table.\n\
         \n\
         Workaround: Use 'qqrl balance' to verify account balances manually.\n\
         \n\
         Pending feature request: https://github.com/rustledger/rustledger/issues/..."
        .into());
}
