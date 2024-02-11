#![allow(dead_code)]
use chrono::{DateTime, Utc};
use ethers::types::U256;
use reqwest::Client;
use serde::Deserialize;
use utils::{deserialize_iso8601_date_time, deserialize_u256_from_json_number_or_string};

static BLASTSCAN_BASE_URL: &str = "https://api.routescan.io/v2/network/testnet/evm/168587773/address/0x841886AB34886FE435Ee8f34b08119f051A40a28/transactions?sort=desc&limit=25";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlastScanItem {
    pub id: String,
    #[serde(deserialize_with = "deserialize_iso8601_date_time")]
    timestamp: DateTime<Utc>,
    pub from: String,
    pub to: String,
    #[serde(deserialize_with = "deserialize_u256_from_json_number_or_string")]
    pub value: U256,
    #[serde(deserialize_with = "deserialize_u256_from_json_number_or_string")]
    pub gas_used: U256,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlastScanNext {
    pub next_token: String,
}

#[derive(Deserialize, Debug)]
pub struct BlastScanResponse {
    pub items: Vec<BlastScanItem>,
    pub link: BlastScanNext,
}

// if gas used is >21000, then it's an erc20 transfer

pub async fn fetch_transactions(
    client: &Client,
    next_token: Option<String>,
) -> anyhow::Result<BlastScanResponse> {
    let url = match next_token {
        Some(token) => format!("{}&nextToken={}", BLASTSCAN_BASE_URL, token),
        None => BLASTSCAN_BASE_URL.to_string(),
    };

    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        let response = response.json::<BlastScanResponse>().await?;
        Ok(response)
    } else {
        Err(anyhow::anyhow!(
            "Failed to fetch transactions: {}",
            response.text().await?
        ))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new();
    let response = fetch_transactions(&client, None).await?;
    println!("{:?}", response);
    let response_2 = fetch_transactions(&client, Some(response.link.next_token)).await?;
    println!("{:?}", response_2);
    Ok(())
}
