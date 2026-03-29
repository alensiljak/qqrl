# Tasks

- [ ] port tests
- [ ] port individual commands
- [x] implement `--exchange/-X` in `bal` command
- [x] handle the differences in `value()` in `lots`
- [x] revert `bal --exchange` to use `convert(sum(position), 'X')` now that rledger supports it
- [x] publish crate
- [x] capitalize `-X` currency as it can be entered lowercase
- [ ] `r l ABC -X AUD` fails due to "error: failed to execute query: unknown function: convert"
- [ ] complete the `query` command after the bug with columns is fixed in Rust Ledger.
- [ ] revert the BQL to use the `IN` operator for currencies after Rust Ledger bug is fixed
