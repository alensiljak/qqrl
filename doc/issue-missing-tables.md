# Missing Tables

`#prices` and `#balances` are missing in Rust Ledger's bean-query.

Beancount:

```txt
beanquery> .tables
accounts
balances
commodities
documents
entries
events
notes
postings
prices
transactions
```

Rust Ledger:

```txt
beanquery> .tables
entries
postings
```
