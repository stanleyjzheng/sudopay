use std::collections::HashMap;

use config::Config;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

static COINBASE_ETH_API: Lazy<String> =
    Lazy::new(|| "https://api.coinbase.com/v2/exchange-rates?currency=ETH".to_string());

#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Asset {
    Eth,
    Usdb,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseDataResponse {
    pub currency: String,
    pub rates: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct CoinbasePriceResponse {
    data: CoinbaseDataResponse,
}

pub struct PriceClient {
    client: reqwest::Client,
    db_pool: PgPool,
}

impl PriceClient {
    pub async fn new(config: Option<Config>, db_pool: Option<PgPool>) -> anyhow::Result<Self> {
        let config = config.unwrap_or(Config::new_from_env());
        let db_pool = db_pool.unwrap_or(PgPool::connect(&config.database_url).await?);

        Ok(Self {
            client: reqwest::Client::new(),
            db_pool,
        })
    }

    pub async fn refresh_eth_price(&self) -> anyhow::Result<()> {
        let resp = self.client.get(&*COINBASE_ETH_API).send().await?;

        if !resp.status().is_success() {
            error!("HTTP request failed with status: {}", resp.status());
            return Err(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                resp.status()
            ));
        }

        match resp.json::<CoinbasePriceResponse>().await {
            Ok(json) => {
                let eth_price_str = json
                    .data
                    .rates
                    .get("USD")
                    .ok_or_else(|| anyhow::anyhow!("USD rate not found"))?; // Convert the error to anyhow::Error

                let eth_price = eth_price_str
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Failed to parse USD rate as f64"))?; // Convert the error to anyhow::Error

                sqlx::query!(
                    "INSERT INTO prices (ticker, price) VALUES ($1, $2) ON CONFLICT (ticker) DO UPDATE SET price = EXCLUDED.price, updated_at = CURRENT_TIMESTAMP",
                    serde_json::to_string(&Asset::Eth)?,
                    eth_price
                )
                .execute(&self.db_pool)
                .await?;
            }
            Err(e) => {
                error!("Failed to parse JSON response: {}", e);
                return Err(e.into());
            }
        };

        info!("Updated ETH price");

        Ok(())
    }

    pub async fn refresh_usdb_price(&self) -> anyhow::Result<()> {
        sqlx::query!(
            "INSERT INTO prices (ticker, price) VALUES ($1, $2) ON CONFLICT (ticker) DO UPDATE SET price = EXCLUDED.price, updated_at = CURRENT_TIMESTAMP",
            serde_json::to_string(&Asset::Usdb)?,
            1.0_f64
        )
        .execute(&self.db_pool)
        .await?;

        info!("Updated USDB price");

        Ok(())
    }

    pub async fn get_cached_price(&self, asset: Asset) -> anyhow::Result<f64> {
        let record = sqlx::query!(
            "SELECT price FROM prices WHERE ticker = $1",
            serde_json::to_string(&asset)?
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(record.price)
    }
}
