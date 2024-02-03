compose:
  sh ./scripts/compose.sh
migrate:
  sqlx migrate run
dev-telegram:
  cargo watch -x 'cargo run --bin telegram'
update-prices:
  cargo run --bin price