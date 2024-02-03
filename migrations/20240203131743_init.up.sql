-- Add up migration script here

CREATE TABLE IF NOT EXISTS users (
  telegram_chat_id SERIAL PRIMARY KEY,
  
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

