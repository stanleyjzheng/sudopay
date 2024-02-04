rust_log := env_var_or_default("RUST_LOG", "debug")

compose:
  sh ./scripts/compose.sh
migrate:
  sqlx migrate run
dev-telegram:
  RUST_LOG={{ rust_log }} cargo watch -x 'cargo run --bin telegram'
update-prices:
  RUST_LOG={{ rust_log }} cargo run --bin price
