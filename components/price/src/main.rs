use config::Config;
use price::PriceClient;

#[tokio::main]
async fn main() {
    let config = Config::new_from_env();
    let price_client = PriceClient::new(Some(config)).await.unwrap();

    price_client.refresh_eth_price().await.unwrap();
    price_client.refresh_usdb_price().await.unwrap();
}
