# qqrl Compatibility Checklist

Verify that all ledger2bql functionality has been correctly ported to qqrl.

## Test Strategy

Run each command against the same source file on both tools and compare output.
Use `tests/sample-ledger.bean` as the shared test ledger.

### Comparison approach

```sh
# Run both, diff the output
ledger2bql <command> [options] > expected.txt
qqrl       <command> [options] > actual.txt
diff expected.txt actual.txt
```

**Normalize before diffing:**

- Column widths and padding may differ between `tabulate` (Python) and `comfy-table` (Rust) — compare content, not formatting
- Decimal display precision may differ (rledger shows `111.11 USD`, Python bean-query may truncate to `111 USD`) — this is an acceptable known difference
- Sort stability for equal values may differ — sort output before diffing if needed

**Recommended diff script:**

```sh
# Strip leading/trailing whitespace per line, then diff
sed 's/^[[:space:]]*//;s/[[:space:]]*$//' expected.txt > e.norm.txt
sed 's/^[[:space:]]*//;s/[[:space:]]*$//' actual.txt   > a.norm.txt
diff e.norm.txt a.norm.txt
```

---

## Global Options

| Option             | ledger2bql | qqrl | Notes               |
|--------------------|------------|------|---------------------|
| `--version`        | ✓          | ✓    |                     |
| `--verbose` / `-v` | ✓          | ☐    | Enable debug output |

---

## Common Options (all commands except `query`)

| Option                | Short | ledger2bql | qqrl | Test case                    |
|-----------------------|-------|------------|------|------------------------------|
| `--begin DATE`        | `-b`  | ✓          | ✓    | `-b 2025-03-01`              |
| `--end DATE`          | `-e`  | ✓          | ✓    | `-e 2025-09-01`              |
| `--date-range RANGE`  | `-d`  | ✓          | ✓    | `-d 2025-03..2025-09`        |
| `--empty`             | —     | ✓          | ☐    | Flag accepted but ignored    |
| `--sort FIELDS`       | `-S`  | ✓          | ✓    | `-S account` / `-S -account` |
| `--limit N`           | —     | ✓          | ✓    | `--limit 3`                  |
| `--amount FILTER`     | `-a`  | ✓          | ✓    | `-a >50EUR`                  |
| `--amount` (multiple) | `-a`  | ✓          | ✓    | `-a >10 -a <200`             |
| `--currency CURR`     | `-c`  | ✓          | ✓    | `-c EUR`                     |
| `--currency (multi)`  | `-c`  | ✓          | ✓    | `-c EUR,BAM`                 |
| `--exchange CURR`     | `-X`  | ✓          | ✓    | `-X EUR`                     |
| `--total`             | `-T`  | ✓          | ✓    | Not supported by `lots`      |
| `--no-pager`          | —     | ✓          | ⚠️    | Pager not implemented (skipped) |

---

## Account Pattern Syntax

| Pattern      | Example                     | ledger2bql | qqrl | Notes                        |
|--------------|-----------------------------|------------|------|------------------------------|
| Plain regex  | `Bank`                      | ✓          | ✓    | Substring match              |
| Starts with  | `^Assets`                   | ✓          | ✓    | BQL: `account ~ '^Assets'`   |
| Ends with    | `Checking$`                 | ✓          | ✓    | BQL: `account ~ 'Checking$'` |
| Exact match  | `^Assets:Bank:Checking$`    | ✓          | ✓    |                              |
| Exclusion    | `Assets not Checking`       | ✓          | ✓    | `not` keyword                |
| Payee filter | `@Employer`                 | ✓          | ✓    | `@` prefix                   |
| Combined     | `Assets not Bank @Employer` | ✓          | ✓    |                              |

---

## Date Range Formats

| Format      | Example                  | ledger2bql | qqrl |
|-------------|--------------------------|------------|------|
| Year only   | `2025`                   | ✓          | ✓    |
| Year-Month  | `2025-03`                | ✓          | ✓    |
| Full date   | `2025-03-15`             | ✓          | ✓    |
| Year range  | `2025..2026`             | ✓          | ✓    |
| Month range | `2025-03..2025-09`       | ✓          | ✓    |
| Day range   | `2025-03-01..2025-09-01` | ✓          | ✓    |
| Open start  | `..2025-09`              | ✓          | ✓    |
| Open end    | `2025-03..`              | ✓          | ✓    |

---

## Amount Filter Formats

| Operator         | Example       | ledger2bql | qqrl |
|------------------|---------------|------------|------|
| Greater than     | `-a >100`     | ✓          | ✓    |
| Greater or equal | `-a >=100EUR` | ✓          | ✓    |
| Less than        | `-a <50USD`   | ✓          | ✓    |
| Less or equal    | `-a <=50`     | ✓          | ✓    |
| Equal            | `-a =1000`    | ✓          | ✓    |
| With currency    | `-a >100EUR`  | ✓          | ✓    |
| Negative amount  | `-a <-100`    | ✓          | ✓    |

---

## `bal` — Account Balances

```sh
# Test commands
ledger2bql bal
ledger2bql bal Assets
ledger2bql bal ^Assets
ledger2bql bal Assets not Bank
ledger2bql bal -b 2025-03-01
ledger2bql bal -e 2025-09-01
ledger2bql bal -d 2025-03..2025-09
ledger2bql bal -c EUR
ledger2bql bal -a >100EUR
ledger2bql bal -T
ledger2bql bal -D 2
ledger2bql bal -Z
ledger2bql bal -H
ledger2bql bal -H -D 2
ledger2bql bal -H -T
ledger2bql bal -S -account
ledger2bql bal --limit 3
ledger2bql bal -X EUR
```

| Feature                   | ledger2bql | qqrl | Notes                                  |
|---------------------------|------------|------|----------------------------------------|
| Basic balances            | ✓          | ✓    |                                        |
| Account filter (regex)    | ✓          | ✓    |                                        |
| Multiple account patterns | ✓          | ✓    |                                        |
| Exclusion (`not`)         | ✓          | ✓    |                                        |
| `--begin` / `--end`       | ✓          | ✓    |                                        |
| `--date-range`            | ✓          | ✓    |                                        |
| `--currency`              | ✓          | ✓    |                                        |
| `--amount`                | ✓          | ✓    | Post-BQL filter on aggregated balances |
| `--total` / `-T`          | ✓          | ✓    | Grand total row                        |
| `--depth` / `-D`          | ✓          | ✓    | Limit account tree depth               |
| `--zero` / `-Z`           | ✓          | ✓    | Exclude zero balances                  |
| `--hierarchy` / `-H`      | ✓          | ✓    | Parent account aggregation             |
| `--sort` / `-S`           | ✓          | ✓    |                                        |
| `--limit`                 | ✓          | ✓    |                                        |
| `--exchange` / `-X`       | ✓          | ✓    | Uses `sum(convert(position, CURR))`    |
| `--no-pager`              | ✓          | ⚠️    | Pager not implemented (skipped)        |

---

## `reg` — Transaction Register

```sh
# Test commands
ledger2bql reg
ledger2bql reg Assets
ledger2bql reg Expenses Food
ledger2bql reg Assets not Bank
ledger2bql reg @Employer
ledger2bql reg -b 2025-03-01
ledger2bql reg -e 2025-09-01
ledger2bql reg -d 2025-08
ledger2bql reg -c EUR
ledger2bql reg -a >50
ledger2bql reg -T
ledger2bql reg -S date
ledger2bql reg -S -date
ledger2bql reg --limit 5
ledger2bql reg -X EUR
```

| Feature                        | ledger2bql | qqrl | Notes                                   |
|--------------------------------|------------|------|-----------------------------------------|
| Basic register                 | ✓          | ✓    | date, account, payee, narration, amount |
| Account filter                 | ✓          | ✓    |                                         |
| Payee filter (`@`)             | ✓          | ✓    |                                         |
| Multiple patterns + exclusions | ✓          | ✓    |                                         |
| `--begin` / `--end`            | ✓          | ✓    |                                         |
| `--date-range`                 | ✓          | ✓    |                                         |
| `--currency`                   | ✓          | ✓    |                                         |
| `--amount`                     | ✓          | ✓    |                                         |
| `--total` / `-T`               | ✓          | ✓    | Running total column                    |
| `--sort` / `-S`                | ✓          | ✓    |                                         |
| `--limit`                      | ✓          | ✓    |                                         |
| `--exchange` / `-X`            | ✓          | ✓    | Uses `convert(position, CURR)`           |
| Multi-currency running totals  | ✓          | ✓    | Tracks each currency separately         |
| `--no-pager`                   | ✓          | ⚠️    | Pager not implemented (skipped)         |

---

## `query` — Named Queries

```sh
# Test commands (requires named query in .bean file)
# sample-ledger.bean contains: 2025-09-02 query "holidays" "select * where ..."
ledger2bql query holidays
ledger2bql query HOLIDAYS      # case-insensitive match
ledger2bql query holi          # partial match
ledger2bql query --list
ledger2bql query --no-pager holidays
```

| Feature                         | ledger2bql | qqrl | Notes                                   |
|---------------------------------|------------|------|-----------------------------------------|
| Exact name match                | ✓          | ✓    |                                         |
| Case-insensitive match          | ✓          | ✓    |                                         |
| Partial match                   | ✓          | ✓    | Falls back when no exact match          |
| Results formatted as table      | ✓          | ✓    |                                         |
| `--list` flag                   | —          | ✓    | List all saved queries                  |
| `--no-pager`                    | ✓          | ⚠️    | Pager not implemented (skipped)         |
| Source: `.bean` file regex scan | —          | ✓    | Replaces `beancount.loader.load_file()` |

---

## `lots` — Investment Lots

```sh
# Test commands
ledger2bql lots
ledger2bql lots Equity
ledger2bql lots --active
ledger2bql lots --all
ledger2bql lots --average
ledger2bql lots --sort-by date
ledger2bql lots --sort-by price
ledger2bql lots --sort-by symbol
ledger2bql lots -b 2025-04-01
ledger2bql lots -c EUR
```

| Feature                    | ledger2bql | qqrl | Notes                            |
|----------------------------|------------|------|----------------------------------|
| Active lots (default)      | ✓          | ✓    | HAVING SUM(units) > 0            |
| All lots (`--all`)         | ✓          | ✓    | Includes sold lots               |
| Average cost (`--average`) | ✓          | ✓    | GROUP BY account+currency        |
| Detailed lots              | ✓          | ✓    | Per-lot date, price, cost; value |
| `--sort-by date`           | ✓          | ✓    |                                  |
| `--sort-by price`          | ✓          | ✓    |                                  |
| `--sort-by symbol`         | ✓          | ✓    |                                  |
| Account filter             | ✓          | ✓    |                                  |
| `--begin` / `--end`        | ✓          | ✓    |                                  |
| `--currency`               | ✓          | ✓    |                                  |
| `cost(position)` column    | ✓          | ✓    |                                  |
| `value(position)` column   | ✓          | ✓    | Fixed in rledger 2026-03-27      |
| `cost_number` column       | ✓          | ✓    | Used for price / average cost    |
| `--no-pager`               | ✓          | ⚠️    | Pager not implemented (skipped)  |

---

## `assert` — Balance Assertions

```sh
# Test commands
ledger2bql assert
ledger2bql assert Checking
ledger2bql assert -b 2025-11-01
ledger2bql assert -e 2025-12-01
ledger2bql assert -c EUR
```

| Feature                   | ledger2bql | qqrl | Notes                                        |
|---------------------------|------------|------|----------------------------------------------|
| List all assertions       | ✓          | ☐    | command returns placeholder error            |
| Account filter            | ✓          | ☐    |                                              |
| Date filter               | ✓          | ☐    |                                              |
| Currency filter           | ✓          | ☐    |                                              |
| Source: `#balances` table | ✓          | ☐    | ⚠️ Phase 0 — rledger may use different syntax |
| `--no-pager`              | ✓          | ⚠️    | Pager not implemented (skipped)               |

---

## `price` — Price History

```sh
# Test commands
ledger2bql price
ledger2bql price EUR
ledger2bql price ABC
ledger2bql price -b 2025-01-01
ledger2bql price -e 2025-06-01
ledger2bql price -d 2025-01
ledger2bql price -c USD
```

| Feature                 | ledger2bql | qqrl | Notes                                        |
|-------------------------|------------|------|----------------------------------------------|
| All prices              | ✓          | ☐    | command returns placeholder error            |
| Symbol filter           | ✓          | ☐    | positional arg                               |
| Date filter             | ✓          | ☐    |                                              |
| Currency filter         | ✓          | ☐    |                                              |
| Source: `#prices` table | ✓          | ☐    | ⚠️ Phase 0 — rledger may use different syntax |
| `--no-pager`            | ✓          | ⚠️    | Pager not implemented (skipped)               |

---

## Phase 0 Risk Items (verify against rledger before porting)

These features are used in ledger2bql but not confirmed to work in rledger. Verify manually with:

```sh
rledger query -f json tests/sample-ledger.bean "QUERY"
```

| Feature                    | Used in                        | BQL query to test                                                   | Status |
|----------------------------|--------------------------------|---------------------------------------------------------------------|--------|
| `convert(position, 'EUR')` | `bal`, `reg` with `--exchange` | `SELECT account, sum(convert(position, 'EUR')) GROUP BY account`    | ✓      |
| `convert(sum(position), 'EUR')` | direct aggregate conversion | `SELECT account, convert(sum(position), 'EUR') GROUP BY account` | ✗ |
| `value(position)`          | `lots`                         | `SELECT account, value(SUM(position)) GROUP BY account`             | ✓      |
| `cost_number`              | `lots`                         | `SELECT account, cost_number WHERE cost_number IS NOT NULL LIMIT 5` | ✓      |
| `#balances` system table   | `assert`                       | `SELECT date, account, amount FROM #balances`                       | ☐      |
| `#prices` system table     | `price`                        | `SELECT date, currency, amount FROM #prices`                        | ☐      |
| JSON schema for Amount     | all                            | Check field names in `-f json` output                               | ✓      |
| JSON schema for Position   | all                            | Check field names in `-f json` output                               | ✓      |
| Android ARM64 binary       | runtime                        | Install and run on Termux                                           | ☐      |

---

## Output Format Notes

These are known acceptable differences between ledger2bql and qqrl output:

| Difference             | ledger2bql                               | qqrl            | Verdict                               |
|------------------------|------------------------------------------|-----------------|---------------------------------------|
| Column padding         | tabulate library                         | comfy-table     | Acceptable — content identical        |
| Decimal precision      | May truncate (beanquery display context) | Full precision  | Acceptable — rledger is more accurate |
| Header separator style | `----`                                   | May differ      | Acceptable                            |
| Empty result           | Empty table or no output                 | ☐               | Must match                            |
| Error messages         | Click/Python style                       | Rust/clap style | Acceptable                            |
