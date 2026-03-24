# Initial Stages

Recommended sequencing for the agent sessions:

## Session 1

Session 1 — Phase 0 (you drive, not the agent)

Do this yourself in a terminal before involving the agent at all:


rledger query -f json tests/sample_ledger.bean "SELECT date, currency, amount FROM #prices"rledger query -f json tests/sample_ledger.bean "SELECT date, account, amount FROM #balances"rledger query -f json tests/sample_ledger.bean "SELECT account, sum(convert(position, 'EUR')) GROUP BY account"rledger query -f json tests/sample_ledger.bean "SELECT account, cost_number WHERE cost_number IS NOT NULL LIMIT 5"rledger query -f json tests/sample_ledger.bean "SELECT account, value(SUM(position)) GROUP BY account"
Record results in COMPATIBILITY.md. This unblocks everything else and prevents the agent from building on wrong assumptions.

## Session 2

Session 2 — Scaffolding (agent)

Single focused task: get a compilable but empty project:

Cargo.toml with all dependencies
src/main.rs with clap command skeleton (all 6 subcommands registered, no logic)
src/config.rs — resolve LEDGER_FILE, RLEDGER_BIN, .env
src/runner.rs — run_bql_query() spawning rledger -f json, parsing JSON, error handling
src/date_parser.rs — port of date_parser.py
src/utils.rs — parse_account_pattern(), parse_amount_filter(), parse_account_params()
Gate: cargo build passes with no logic yet.

## Session 3+

Session 3+ — One command per session, easiest to hardest:

price — simplest query, simple output, good warm-up
assert — similarly simple (pending Phase 0 findings on #balances)
query — simple output, but .bean file regex scan is interesting logic
bal — more output logic; do basic first, then --hierarchy
reg — running totals, multi-currency
lots — last, most complex, depends on Phase 0 findings
Each session: port command → port tests → cargo test green → tick off COMPATIBILITY.md checkboxes.

Why this order matters: runner.rs and utils.rs from Session 2 are shared by everything. Building those first means Sessions 3+ are purely about the command logic, not infrastructure. Doing the hardest command (lots) last means its Phase 0 risks are fully understood before any code is written for it.