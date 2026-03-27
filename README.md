# qqrl

Quick Query for Rust Ledger

<img src="https://img.shields.io/crates/v/qqrl.svg" alt="Crates.io">
<img src="https://docs.rs/qqrl/badge.svg" alt="Docs.rs">

## Purpose

`qqrl` is a rewrite of [ledger2bql](https://github.com/alensiljak/ledger2bql) with the change of underlying engine from Python Beancount to Rust Ledger ([repo](https://github.com/rustledger/rustledger)).

It provides a fast CLI that translates Ledger CLI syntax to BQL queries, executes them via `rledger`, and formats the output.

## Installation

```sh
cargo install qqrl
```

## Development

Build from source:

```sh
cargo install --path .
```

Run tests:

```sh
cargo test
```

## Quick Start

1. Set your ledger file path:

```sh
export LEDGER_FILE=/path/to/your/ledger.bean
```

2. Try a basic query:

```sh
qqrl bal
qqrl reg Assets:Bank
qqrl q --list
```

## Documentation

- **[Full Usage Guide](doc/usage.md)** — comprehensive command reference with examples
- [Rust Ledger documentation](https://rustledger.github.io/about/why-rustledger.html)
- [BQL reference](https://beancount.github.io/docs/beancount_query_language.html)
- [Compatibility Report](doc/COMPATIBILITY.md)

## Commands Overview

| Command | Aliases | Description |
|---------|---------|-------------|
| `bal` | `b` | Account balances |
| `reg` | `r` | Transaction register |
| `query` | `q` | Execute named queries from ledger |
| `lots` | `l` | Investment lots / cost basis |
| `assert` | `a` | Balance assertions *(blocked)* |
| `price` | `p` | Price history *(blocked)* |

All commands support common options like `--begin`, `--end`, `--currency`, `--exchange`, `--sort`, `--limit`, and `--total`.

## Configuration

Set environment variables (or use a `.env` file):

- `LEDGER_FILE` or `BEANCOUNT_FILE` — path to `.bean` ledger file (required)
- `RLEDGER_BIN` — custom path to `rledger` binary (optional, default: `rledger`)

## Links

- Ledger2Bql [repo](https://github.com/alensiljak/ledger2bql)
- Rust Ledger [repo](https://github.com/rustledger/rustledger)
- Rust Ledger [releases](https://github.com/rustledger/rustledger/releases/latest)

## License

AGPL-3.0
