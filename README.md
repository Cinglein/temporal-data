# Temporal Takehome

## Incipit

This program tracks SolanaMEVBot `MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz` trades to look at its profitability.

The database is defined in the `migrations` folder. I write everything to a `txs` table that tracks:

- Feepayer wallet
- TX signature
- Blocktime
- Slot
- Fee
- Raw arb profit

The program is currently deployed to a EC2 box and writes to a NeonDB database. There are three Grafana views which track net profitability per trade, trades per hour, and net profitability per hour.
