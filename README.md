# qqrl

Quick Query for Rust Ledger

<img src="https://img.shields.io/crates/v/qqrl.svg" alt="Crates.io">
<img src="https://docs.rs/qqrl/badge.svg" alt="Docs.rs">

## Purpose

`qqrl` is a rewrite of [ledger2bql](https://github.com/alensiljak/ledger2bql) with the change of underlying engine from Python Beancount to Rust Ledger.

## Installation

For the moment, the app can only be installed by building from the source, as it is in in a very early stage of development.

```sh
cargo install --path .
```

## Development

Run tests:

```sh
cargo tests
```

## Links

- Ledger2Bql [repo](https://github.com/alensiljak/ledger2bql)
- Rust Ledger [repo](https://github.com/rustledger/rustledger)
- Rust Ledger [documentation](https://rustledger.github.io/about/why-rustledger.html)
- BQL [reference](https://beancount.github.io/docs/beancount_query_language.html)
