# SudoPay

a hacky side project built in a few days; will probably not maintain it nor release it.

follows [polylith](https://polylith.gitbook.io/polylith/)

## FAQ
- Why use a monolithic contract?
    - Gas isn’t claimable otherwise; if this becomes a problem, it’s fairly trivial for us to switch to EOA’s like a typical telegram bot, or CEX deposit address. We already have the database fields to do this (ie, we store a “seed phrase”) as an intermediary between telegram_id and balances for account export purposes, but this can be repurposed for an EOA (with added security, of course)
- Wen launch?
    - SudoPay doesn't really make any sense to run on a testnet, so it'll launch with Blast mainnet (alongside many more in-development features).

## Instructions to run locally
- `cargo install sqlx-cli` if you haven't already
- Populate .env.example and rename to .env
- Add `just update-prices` to crontab (every 5min)
- `cargo run --bin telegram` to run the telegram bot
- `cargo run --bin transaction_listener` to run the deposits listener
