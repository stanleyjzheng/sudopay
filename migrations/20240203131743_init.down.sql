-- Add down migration script here

-- Prices
DROP TRIGGER IF EXISTS updated_modified_trigger ON prices;
DROP FUNCTION IF EXISTS trigger_set_updated_timestamp();
DROP TABLE IF EXISTS prices;
