-- Add up migration script here

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

CREATE TRIGGER updated_modified_trigger
BEFORE UPDATE ON prices
FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_timestamp();
