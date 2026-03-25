rledger query -f json tests/sample-ledger.bean "SELECT date, currency, amount FROM #prices"
rledger query -f json tests/sample-ledger.bean "SELECT date, account, amount FROM #balances"
:: Supported row-level conversion aggregated afterwards
rledger query -f json tests/sample-ledger.bean "SELECT account, sum(convert(position, 'EUR')) GROUP BY account"
:: Unsupported aggregate conversion shape; should currently fail with unknown function: convert
rledger query -f json tests/sample-ledger.bean "SELECT account, convert(sum(position), 'EUR') GROUP BY account"
:: qqrl balance exchange workaround/query shape
rledger query -f json tests/sample-ledger.bean "SELECT account, units(sum(position)) as Balance, sum(convert(position, 'EUR')) as Converted GROUP BY account"
:: qqrl register exchange query shape
rledger query -f json tests/sample-ledger.bean "SELECT date, account, payee, narration, position, convert(position, 'EUR') as Converted WHERE account = 'Assets:Bank:Bank03581'"
:: End-to-end qqrl exchange probes
qqrl bal --ledger tests/sample-ledger.bean --exchange EUR Assets:Bank:Bank03581
qqrl reg --ledger tests/sample-ledger.bean --exchange EUR Assets:Bank:Bank03581
qqrl reg --ledger tests/sample-ledger.bean --exchange EUR --total Assets:Bank:Bank03581
rledger query -f json tests/sample-ledger.bean "SELECT account, cost_number WHERE cost_number IS NOT NULL LIMIT 5"
rledger query -f json tests/sample-ledger.bean "SELECT account, value(SUM(position)) GROUP BY account"