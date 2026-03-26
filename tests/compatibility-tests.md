# Compatibility tests

Run with

```sh
scrut test compatibility-tests.md
```

```scrut
$ cat $TESTDIR
some
```

```scrut
$ ledger2bql b --limit 10
Your BQL query is:
SELECT account, units(sum(position)) as Balance ORDER BY account ASC LIMIT 10

+--------------------------+----------------------+
| Account                  |              Balance |
|--------------------------+----------------------|
| Assets:Bank:Bank03581    |         3,000.00 CHF |
| Assets:Bank:Checking     |         1,369.80 EUR |
| Assets:Bank:Savings      |           500.00 EUR |
| Assets:Cash:BAM          |           -25.00 BAM |
| Assets:Cash:Pocket-Money |           -45.00 EUR |
| Assets:Cash:USD          |            -7.00 USD |
| Equity:Opening-Balances  |        -1,000.00 EUR |
| Equity:Stocks            |             4.00 ABC |
| Expenses:Accommodation   |            25.00 EUR |
| Expenses:Food            | 100.00 EUR 25.00 BAM |
+--------------------------+----------------------+
```

Run qqrl:

```scrut
$ qqrl b --limit 10
Your BQL query is:
SELECT account, units(sum(position)) as Balance GROUP BY account ORDER BY account ASC LIMIT 10

┌──────────────────────────┬──────────────────────┐
│ Account                  ┆              Balance │
╞══════════════════════════╪══════════════════════╡
│ Assets:Bank:Bank03581    ┆         3,000.00 CHF │
│ Assets:Bank:Checking     ┆         1,369.80 EUR │
│ Assets:Bank:Savings      ┆           500.00 EUR │
│ Assets:Cash:BAM          ┆           -25.00 BAM │
│ Assets:Cash:Pocket-Money ┆           -45.00 EUR │
│ Assets:Cash:USD          ┆            -7.00 USD │
│ Equity:Opening-Balances  ┆        -1,000.00 EUR │
│ Equity:Stocks            ┆             4.00 ABC │
│ Expenses:Accommodation   ┆            25.00 EUR │
│ Expenses:Food            ┆ 100.00 EUR 25.00 BAM │
└──────────────────────────┴──────────────────────┘
```
