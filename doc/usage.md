# qqrl Usage Guide

Quick Query for Rust Ledger — comprehensive command reference

## Table of Contents

- [qqrl Usage Guide](#qqrl-usage-guide)
  - [Table of Contents](#table-of-contents)
  - [Configuration](#configuration)
    - [Environment Variables](#environment-variables)
    - [.env File Support](#env-file-support)
  - [Common Options](#common-options)
  - [Commands](#commands)
    - [Balance (`bal`, `b`)](#balance-bal-b)
    - [Register (`reg`, `r`)](#register-reg-r)
    - [Query (`q`)](#query-q)
    - [Lots (`lots`, `l`)](#lots-lots-l)
    - [Assert (`a`)](#assert-a)
    - [Price (`p`)](#price-p)
  - [Account Pattern Matching](#account-pattern-matching)
  - [Date Formats](#date-formats)
    - [Single Dates](#single-dates)
    - [Date Ranges](#date-ranges)
    - [Usage with Options](#usage-with-options)
  - [Amount Filters](#amount-filters)
  - [Output](#output)
    - [Table Format](#table-format)
    - [Pager](#pager)
  - [Troubleshooting](#troubleshooting)
    - ["LEDGER\_FILE or BEANCOUNT\_FILE environment variable not set"](#ledger_file-or-beancount_file-environment-variable-not-set)
    - ["rledger not found"](#rledger-not-found)
    - [Command-specific issues](#command-specific-issues)
  - [Links](#links)

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `LEDGER_FILE` | Yes | Path to your `.bean` ledger file |
| `BEANCOUNT_FILE` | Alternative | Alternative to `LEDGER_FILE` for backward compatibility |
| `RLEDGER_BIN` | No | Path to the `rledger` binary (default: `rledger`) |

### .env File Support

Create a `.env` file in your project directory:

```env
LEDGER_FILE=/path/to/your/ledger.bean
RLEDGER_BIN=/custom/path/to/rledger
```

The `.env` file is automatically loaded if present.

## Common Options

All commands support these options:

| Option | Short | Description |
|--------|-------|-------------|
| `--ledger PATH` | | Override ledger file path (overrides env var) |
| `--begin DATE` | `-b` | Start date (YYYY-MM-DD or date range format) |
| `--end DATE` | `-e` | End date (YYYY-MM-DD or date range format) |
| `--date-range RANGE` | `-d` | Date range (YYYY-MM..YYYY-MM or YYYY-MM-DD..YYYY-MM-DD) |
| `--amount FILTER` | `-a` | Amount filter(s) — e.g., `>100EUR`, `<=50USD` (can be repeated) |
| `--currency CODE` | `-c` | Currency filter(s) — e.g., `EUR` or `EUR,USD` (can be repeated) |
| `--exchange CODE` | `-X` | Exchange currency — convert all amounts to this currency |
| `--sort FIELDS` | `-S` | Sort by field(s) — prefix with `-` for descending (e.g., `account`, `-amount`, `date account`) |
| `--limit N` | | Limit number of results |
| `--total` | `-T` | Show running total / summary |
| `--no-pager` | | Disable pager output |

## Commands

### Balance (`bal`, `b`)

Show account balances.

**Additional options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--hierarchy` | `-H` | Show account hierarchy (expands parent accounts) |
| `--empty` | | Include empty accounts |
| `--depth N` | `-D` | Limit account tree depth |
| `--zero` | `-Z` | Exclude accounts with zero balance |

**Examples:**

```sh
# Show all account balances
qqrl bal

# Show balances with hierarchy (parent accounts aggregate children)
qqrl bal --hierarchy

# Filter by account pattern
qqrl bal Assets:Bank

# Convert all amounts to EUR
qqrl bal -X EUR

# Show only EUR and USD accounts
qqrl bal -c EUR -c USD

# Limit depth to 2 levels
qqrl bal --hierarchy --depth 2

# Exclude zero balances
qqrl bal --zero

# Combine filters
qqrl bal -b 2025-01-01 -e 2025-12-31 Assets -X EUR --total
```

**How it works:**

The balance command aggregates positions by account using `sum(position)` and optionally converts to a target currency using `convert(position, 'CURRENCY')`.

With `--hierarchy`, parent accounts are computed by aggregating all child account balances.

### Register (`reg`, `r`)

Show transaction register (detailed posting history).

**Examples:**

```sh
# Show all transactions
qqrl reg

# Filter by account and date range
qqrl reg Assets:Bank -b 2025-01-01 -e 2025-03-31

# Filter by amount
qqrl reg -a '>100' -a '<1000'

# Sort by date descending
qqrl reg -S -date

# Show running total
qqrl reg -T

# Convert to EUR
qqrl reg -X EUR
```

**How it works:**

The register command selects individual postings with `SELECT date, account, payee, narration, position`. Amount filters are applied directly in the BQL WHERE clause. Running totals are computed client-side by qqrl.

### Query (`q`)

Execute named queries from your ledger file.

**Usage:**

```sh
# List all available saved queries
qqrl query --list

# Execute a query by name
qqrl query QUERY_NAME
qqrl q holidays
qqrl q my-custom-report
```

**How it works:**

The command scans your ledger file for lines containing `query "name" "BQL_STATEMENT"` directives:

```ledger
2025-09-02 query "holidays" "SELECT * WHERE payee ~ 'holiday'"
2025-09-03 query "monthly-summary" "SELECT account, sum(position) GROUP BY account"
```

Query name matching follows this hierarchy:
1. Exact match (case-sensitive)
2. Case-insensitive match
3. Partial match (contains, case-insensitive)

### Lots (`lots`, `l`)

Show investment lots and cost basis.

**Additional options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--sort-by` | `-s` | Sort lots by: `date`, `price`, or `symbol` |
| `--average` | `-A` | Show average cost for each symbol |
| `--all` | | Show all lots, including sold/closed ones |
| `--closed` | | Show only closed/inactive lots |
| `--active` | | Show only active/open lots (default) |

**Examples:**

```sh
# Show active investment lots
qqrl lots

# Show all lots (including sold)
qqrl lots --all

# Show average cost by symbol
qqrl lots --average

# Sort by acquisition date
qqrl lots -s date

# Filter by account and currency
qqrl lots Assets:Investments -c USD

# Convert values to EUR
qqrl lots -X EUR
```

**Output columns:**

- **Default (active lots):** Date, Account, Symbol, Quantity, Price, Cost
- **With `--all`:** Date, Account, Symbol, Quantity, Price, Cost, Value
- **With `--average`:** Date, Account, Symbol, Quantity, Average Price, Total Cost, Value

**Note:** The market value (`Value`) column may be limited depending on `rledger`'s `value(...)` function support. See [Compatibility Report](COMPATIBILITY.md) for current status.

### Assert (`a`)

Display and verify balance assertions.

**⚠️  Status: BLOCKED**

This command requires `rledger` to support the `#balances` system table. It is not yet available.

**Workaround:** Use `qqrl bal` to manually verify account balances.

**Pending feature request:** https://github.com/rustledger/rustledger/issues/...

**Usage:**

```sh
qqrl assert [PATTERN] [OPTIONS]
qqrl a Assets
qqrl assert --date 2025-12-31
```

### Price (`p`)

Display price history.

**⚠️  Status: BLOCKED**

This command requires `rledger` to support the `#prices` system table. It is not yet available.

**Pending feature request:** https://github.com/rustledger/rustledger/issues/...

**Usage:**

```sh
qqrl price [COMMODITY] [OPTIONS]
qqrl p EUR
qqrl price --begin 2025-01-01 USD
```

## Account Pattern Matching

Account arguments support flexible pattern matching:

| Pattern | Meaning |
|---------|---------|
| `Assets:Bank` | Exact match |
| `Assets` | Matches any account starting with "Assets" |
| `^Assets` | Regex: starts with "Assets" |
| `:Bank$` | Regex: ends with "Bank" |
| `Assets not Bank` | Exclude accounts containing "Bank" |
| `Assets @Employer` | Multiple patterns (OR logic) |

**Examples:**

```sh
qqrl bal Assets:Bank           # Exact account
qqrl bal Assets               # All accounts under Assets
qqrl bal '^Assets:'           # All accounts starting with Assets:
qqrl bal ':Checking$'         # All accounts ending with Checking
qqrl bal Assets not Bank      # Assets accounts but not Bank
qqrl bal Assets @Income       # Assets OR Income accounts
```

## Date Formats

### Single Dates

- `2025-03-15` — March 15, 2025
- `2025-03` — March 1, 2025 (month start)

### Date Ranges

- `2025-01..2025-12` — entire year 2025
- `2025-01-01..2025-12-31` — specific date range
- `2025-03..2025-06` — March 1 to June 1

**Note:** End dates are exclusive (upper bound). For inclusive end dates, use the day after.

### Usage with Options

```sh
# Using --begin and --end
qqrl bal -b 2025-01-01 -e 2025-12-31

# Using --date-range (single argument)
qqrl bal -d 2025-01..2025-12

# Mixed with other filters
qqrl reg -b 2025-03 -e 2025-04 Assets -T
```

## Amount Filters

Amount filters use comparison operators: `=`, `>`, `>=`, `<`, `<=`, `!=`

**Syntax:**

```
[operator][number][currency]
```

**Examples:**

- `>100EUR` — greater than 100 Euros
- `<=50USD` — less than or equal to 50 US Dollars
- `=0` — zero amount (any currency)
- `>100` — greater than 100 (any currency)
- `!=0USD` — not equal to 0 US Dollars

**Usage:**

```sh
# Multiple amount filters (AND logic)
qqrl bal -a '>100' -a '<1000'

# With currency
qqrl reg -a '>50EUR'

# Zero balance check
qqrl bal -a '=0'
```

## Output

All commands display formatted tables. The BQL query being executed is always printed first for transparency and debugging.

### Table Format

- Columns are aligned for readability
- Multi-currency amounts shown in separate columns
- Decimal numbers use standard notation (no thousands separators)
- Dates in YYYY-MM-DD format

### Pager

By default, long output may be piped through a pager (if configured in your environment). Use `--no-pager` to disable this behavior.

## Troubleshooting

### "LEDGER_FILE or BEANCOUNT_FILE environment variable not set"

Set the environment variable to point to your ledger file:

```sh
export LEDGER_FILE=/path/to/your/ledger.bean
```

Or use `--ledger` option to specify the file on the command line:

```sh
qqrl bal --ledger /path/to/your/ledger.bean
```

### "rledger not found"

Ensure `rledger` is installed and available on your `PATH`. Install from [RustLedger releases](https://github.com/rustledger/rustledger/releases/latest) or via package manager:

```sh
# Using cargo
cargo install rustledger

# Using scoop (Windows)
scoop install rustledger

# Using homebrew (macOS)
brew install rustledger
```

To use a custom path, set `RLEDGER_BIN`:

```sh
export RLEDGER_BIN=/custom/path/to/rledger
```

### Command-specific issues

- **Lots market value missing:** The `value()` function in `rledger` may not return usable data yet. This is a known limitation. See [Compatibility Report](COMPATIBILITY.md).
- **Assert/Price commands blocked:** These require `#balances` and `#prices` system tables which are not yet supported in `rledger`. Track the upstream issue for updates.

## Links

- Project README: [../README.md](../README.md)
- Rust Ledger: https://github.com/rustledger/rustledger
- BQL Reference: https://rustledger.github.io/reference/bql.html
- Compatibility Report: [COMPATIBILITY.md](COMPATIBILITY.md)
