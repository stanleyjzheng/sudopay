-- Add down migration script here

-- Users
DROP TRIGGER IF EXISTS users_updated_modified_trigger ON prices;
DROP TABLE IF EXISTS users;

-- Balances
DROP TRIGGER IF EXISTS balances_updated_modified_trigger ON prices;
DROP TABLE IF EXISTS balances;

-- Transactions
DROP TRIGGER IF EXISTS transactions_updated_modified_trigger ON prices;
DROP TABLE IF EXISTS transactions;

-- Prices
DROP TRIGGER IF EXISTS prices_updated_modified_trigger ON prices;
DROP FUNCTION IF EXISTS trigger_set_updated_timestamp();
DROP TABLE IF EXISTS prices;
