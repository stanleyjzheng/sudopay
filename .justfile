rust_log := env_var_or_default("RUST_LOG", "debug")

compose:
  sh ./scripts/compose.sh
migrate:
  sqlx migrate run
dev-telegram:
  just update-prices && RUST_LOG={{ rust_log }} cargo watch -x 'run --bin telegram'
update-prices:
  RUST_LOG={{ rust_log }} cargo run --bin price
purge-sql:
  sqlx migrate revert && sqlx migrate run
transaction-listener:
  RUST_LOG={{ rust_log }} cargo run --bin transaction_listener
