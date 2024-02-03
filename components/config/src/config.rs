use dotenv::dotenv;
use std::env;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub database_url: String,
    pub teloxide_token: String,
}

impl Config {
    pub fn new_from_env() -> Config {
        dotenv().expect("Failed to load .env file");

        let teloxide_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN");
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL");

        Config {
            database_url,
            teloxide_token,
        }
    }
}
