use dotenv::dotenv;
use std::env;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub database_url: String,
    pub teloxide_token: String,
    pub transferrer_private_key: String,
    pub contract_address: String,
    pub http_rpc_url: String,
    pub mainnet_http_rpc_url: String,
    pub chain_id: u64,
}

impl Config {
    pub fn new_from_env() -> Config {
        dotenv().expect("Failed to load .env file");

        let teloxide_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN");
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL");
        let transferrer_private_key =
            env::var("TRANSFERRER_PRIVATE_KEY").expect("TRANSFERRER_PRIVATE_KEY");
        let contract_address = env::var("CONTRACT_ADDRESS")
            .expect("CONTRACT_ADDRESS")
            .to_lowercase();
        let http_rpc_url = env::var("HTTP_RPC_URL").expect("HTTP_RPC_URL");
        let chain_id = env::var("CHAIN_ID")
            .expect("CHAIN_ID")
            .parse::<u64>()
            .expect("CHAIN_ID must be a number");
        let mainnet_http_rpc_url = env::var("MAINNET_HTTP_RPC_URL").expect("MAINNET_HTTP_RPC_URL");

        Config {
            database_url,
            teloxide_token,
            transferrer_private_key,
            contract_address,
            mainnet_http_rpc_url,
            http_rpc_url,
            chain_id,
        }
    }
}
