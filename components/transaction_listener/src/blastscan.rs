#![allow(dead_code)]
use chrono::{DateTime, Utc};
use common::utils::{
    deserialize_iso8601_date_time, deserialize_u256_from_json_number_or_string, CONTRACT_ADDRESS,
};
use ethers::types::U256;
use reqwest::Client;
use serde::Deserialize;

static BLASTSCAN_ACCOUNT_URL: &str = "https://api.routescan.io/v2/network/testnet/evm/168587773/address/0x841886AB34886FE435Ee8f34b08119f051A40a28/transactions?sort=desc&limit=100";
static BLASTSCAN_ERC20_URL: &str = "https://api.routescan.io/v2/network/testnet/evm/168587773/address/0x841886AB34886FE435Ee8f34b08119f051A40a28/erc20-transfers?sort=desc&limit=100";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastScanErc20Item {
    pub tx_hash: String,
    #[serde(deserialize_with = "deserialize_iso8601_date_time")]
    pub created_at: DateTime<Utc>,
    pub from: String,
    pub to: String,
    pub token_address: String,
    #[serde(deserialize_with = "deserialize_u256_from_json_number_or_string")]
    pub amount: U256,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastScanTransactionsItem {
    pub id: String,
    #[serde(deserialize_with = "deserialize_iso8601_date_time")]
    pub created_at: DateTime<Utc>,
    pub from: String,
    pub to: String,
    #[serde(deserialize_with = "deserialize_u256_from_json_number_or_string")]
    pub value: U256,
    #[serde(deserialize_with = "deserialize_u256_from_json_number_or_string")]
    pub gas_used: U256,
}

#[derive(Deserialize, Debug)]
pub(crate) struct BlastScanErc20Response {
    pub items: Vec<BlastScanErc20Item>,
    pub link: BlastScanNext,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlastScanNext {
    pub next_token: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct BlastScanTransactionsResponse {
    pub items: Vec<BlastScanTransactionsItem>,
    pub link: BlastScanNext,
}

pub(crate) async fn list_eth_transfers(
    client: &Client,
    next_token: Option<String>,
) -> anyhow::Result<BlastScanTransactionsResponse> {
    let url = match next_token {
        Some(token) => format!("{}&nextToken={}", BLASTSCAN_ACCOUNT_URL, token),
        None => BLASTSCAN_ACCOUNT_URL.to_string(),
    };

    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        let response = response.json::<BlastScanTransactionsResponse>().await?;

        // filter zero value, since there will be erc20 transactions here too.
        let response = BlastScanTransactionsResponse {
            items: response
                .items
                .into_iter()
                .filter(|item| {
                    item.value > U256::zero()
                        && item.to.to_lowercase() == format!("{:#?}", CONTRACT_ADDRESS)
                })
                .collect(),
            link: response.link,
        };

        Ok(response)
    } else {
        Err(anyhow::anyhow!(
            "Failed to fetch transactions: {}",
            response.text().await?
        ))
    }
}

pub(crate) async fn list_erc20_transfers(
    client: &Client,
    next_token: Option<String>,
) -> anyhow::Result<BlastScanErc20Response> {
    let url = match next_token {
        Some(token) => format!("{}&nextToken={}", BLASTSCAN_ERC20_URL, token),
        None => BLASTSCAN_ERC20_URL.to_string(),
    };

    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        let response = response.json::<BlastScanErc20Response>().await?;

        let response = BlastScanErc20Response {
            items: response
                .items
                .into_iter()
                .filter(|item| item.to.to_lowercase() == format!("{:#?}", CONTRACT_ADDRESS))
                .collect(),
            link: response.link,
        };

        Ok(response)
    } else {
        Err(anyhow::anyhow!(
            "Failed to fetch erc20's: {}",
            response.text().await?
        ))
    }
}
