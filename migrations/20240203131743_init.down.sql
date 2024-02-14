-- Add down migration script here

-- Deposit requests
DROP TRIGGER IF EXISTS deposit_requests_updated_modified_trigger ON deposit_requests;
DROP TABLE IF EXISTS deposit_requests;

-- Deposits
DROP TRIGGER IF EXISTS deposits_updated_modified_trigger ON deposits;
DROP TABLE IF EXISTS deposits;

-- Users
DROP TRIGGER IF EXISTS users_updated_modified_trigger ON users;
DROP TABLE IF EXISTS users;

-- Balances
DROP TRIGGER IF EXISTS balances_updated_modified_trigger ON balances;
DROP TABLE IF EXISTS balances;

-- Transactions
DROP TRIGGER IF EXISTS transactions_updated_modified_trigger ON transactions;
DROP TABLE IF EXISTS transactions;

-- Prices
DROP TRIGGER IF EXISTS prices_updated_modified_trigger ON prices;
DROP FUNCTION IF EXISTS trigger_set_updated_timestamp();
DROP TABLE IF EXISTS prices;
