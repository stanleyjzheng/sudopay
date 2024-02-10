-- Add up migration script here

-- Users
CREATE TABLE IF NOT EXISTS users (
  telegram_id BIGINT PRIMARY KEY NOT NULL,
  salted_password TEXT DEFAULT NULL,
  seed_phrase TEXT NOT NULL,
  seed_phrase_public_key TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Wallet balances
CREATE TABLE IF NOT EXISTS balances (
  public_key TEXT PRIMARY KEY NOT NULL,
  usdb_balance double precision NOT NULL DEFAULT 0,
  eth_balance double precision NOT NULL DEFAULT 0,
  accrued_yield_balance double precision NOT NULL DEFAULT 0
);

-- Internal transactions
CREATE TABLE IF NOT EXISTS transactions (
  id SERIAL PRIMARY KEY NOT NULL,
  from_public_key TEXT NOT NULL,
  to_public_key TEXT NOT NULL,
  amount double precision NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Withdrawals

-- Address deposits

-- Balance deposits

-- 

-- Prices
CREATE TABLE IF NOT EXISTS prices (
  ticker TEXT PRIMARY KEY NOT NULL,
  price double precision NOT NULL,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE OR REPLACE FUNCTION trigger_set_updated_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Users table trigger
DROP TRIGGER IF EXISTS users_updated_modified_trigger ON users;
CREATE TRIGGER users_updated_modified_trigger
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_timestamp();

-- Balances table trigger
DROP TRIGGER IF EXISTS balances_updated_modified_trigger ON balances;
CREATE TRIGGER balances_updated_modified_trigger
BEFORE UPDATE ON balances
FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_timestamp();

-- Transactions table trigger
DROP TRIGGER IF EXISTS transactions_updated_modified_trigger ON transactions;
CREATE TRIGGER transactions_updated_modified_trigger
BEFORE UPDATE ON transactions
FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_timestamp();

-- Prices table trigger
DROP TRIGGER IF EXISTS prices_updated_modified_trigger ON prices;
CREATE TRIGGER prices_updated_modified_trigger
BEFORE UPDATE ON prices
FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_timestamp();