mod blastscan;

use chrono::Utc;
use common::{
    types::SudoPayAsset,
    utils::{u256_to_big_decimal, TOKEN_ADDRESS_TO_ASSET},
};
use db::deposits::{Deposit, DepositRequest, NewDeposit};
use itertools::Itertools;
use once_cell::sync::Lazy;
use reqwest::Client;
use sqlx::PgPool;
use tokio::time::{sleep, Duration};

use crate::blastscan::{list_erc20_transfers, list_eth_transfers};

static DEPOSIT_REQUEST_DURATION_SECONDS: i64 = 180;
static DEPOSIT_ANTI_FRONTRUN_DURATION_SECONDS: i64 = 3;

// TODO: CEX withdrawal addresses upon mainnet, or an automated process to pull walletlabels
static CEX_WITHDRAWAL_ADDRESSES: Lazy<Vec<String>> =
    Lazy::new(|| vec!["0xc0ffeebabe000000000000000000000000000000".to_string()]);

async fn match_deposits_to_user_deposit_requests(
    pool: &PgPool,
    new_deposit: NewDeposit,
) -> anyhow::Result<()> {
    let start_time = new_deposit.created_at
        - chrono::Duration::seconds(
            DEPOSIT_REQUEST_DURATION_SECONDS + DEPOSIT_ANTI_FRONTRUN_DURATION_SECONDS,
        );
    let current_utc = Utc::now();
    let end_time = current_utc - chrono::Duration::seconds(DEPOSIT_ANTI_FRONTRUN_DURATION_SECONDS);

    let deposit_addresses = DepositRequest::from_address_by_time(
        pool,
        format!("{:#?}", new_deposit.transaction_from_public_key),
        new_deposit.asset,
        start_time,
        end_time,
    )
    .await?;

    let non_cex_deposit_addresses = deposit_addresses
        .into_iter()
        .filter(|deposit| {
            !CEX_WITHDRAWAL_ADDRESSES.contains(&deposit.from_address.clone().unwrap_or_default())
        })
        .collect::<Vec<DepositRequest>>();

    let unique_deposit_addresses = non_cex_deposit_addresses
        .iter()
        .map(|deposit| deposit.depositor_public_key.clone())
        .unique()
        .collect::<Vec<String>>();

    match unique_deposit_addresses.len() {
        0 => {}
        1 => {
            // we matched a deposit
            Deposit::set_deposit_matched(pool, &new_deposit.transaction_id.clone()).await?;
            DepositRequest::set_matched_transaction_id(
                pool,
                non_cex_deposit_addresses[0].id,
                new_deposit.transaction_id.clone(),
            )
            .await?;
        }
        _ => {
            // we matched multiple deposits
            log::error!(
                "Multiple deposit requests matched a single deposit: {:?}",
                unique_deposit_addresses
            );
        }
    };

    Ok(())
}

async fn fetch_new_eth_deposits(pool: &mut PgPool, client: &Client) -> anyhow::Result<()> {
    let mut next_token: Option<String> = None;
    let mut all_eth_transfer_ids: Vec<String> = Vec::new();

    loop {
        let eth_transfer_response = list_eth_transfers(&client, next_token.clone()).await?;
        let current_page_ids = eth_transfer_response
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<String>>();
        all_eth_transfer_ids.extend(current_page_ids.clone());

        if Deposit::all_transaction_ids_exist(pool, &all_eth_transfer_ids).await? {
            break;
        }

        let new_transaction_ids =
            Deposit::filter_non_existing_transaction_ids(pool, &current_page_ids).await?;

        let new_deposits = eth_transfer_response
            .items
            .into_iter()
            .filter(|item| new_transaction_ids.contains(&item.id))
            .map(|item| NewDeposit {
                transaction_id: item.id,
                amount: u256_to_big_decimal(item.value),
                transaction_from_public_key: item.from,
                asset: SudoPayAsset::Eth,
                created_at: item.created_at,
            })
            .collect::<Vec<_>>();

        Deposit::insert_bulk_deposits(pool, new_deposits).await?;

        next_token = Some(eth_transfer_response.link.next_token);

        // Blastscan minimum req/s is 1, so we sleep for 1s after each request. probably fine for now since we won't be hitting
        // 100 eth/erc20 deposits/s for a while. Or probably ever.
        // TODO: Exponential backoff instead of sleep
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

async fn fetch_new_erc20_deposits(pool: &mut PgPool, client: &Client) -> anyhow::Result<()> {
    let mut next_token: Option<String> = None;
    let mut all_erc20_transfer_ids: Vec<String> = Vec::new();

    loop {
        let erc20_transfer_response = list_erc20_transfers(&client, next_token.clone()).await?;
        let current_page_ids = erc20_transfer_response
            .items
            .iter()
            .map(|item| item.tx_hash.clone())
            .collect::<Vec<String>>();
        all_erc20_transfer_ids.extend(current_page_ids.clone());

        if Deposit::all_transaction_ids_exist(pool, &all_erc20_transfer_ids).await? {
            break;
        }

        let new_transaction_ids =
            Deposit::filter_non_existing_transaction_ids(pool, &current_page_ids).await?;

        let new_deposits = erc20_transfer_response
            .items
            .into_iter()
            .filter(|item| new_transaction_ids.contains(&item.tx_hash))
            .filter_map(
                |item| match TOKEN_ADDRESS_TO_ASSET.get(&item.token_address) {
                    Some(asset) => Some(NewDeposit {
                        transaction_id: item.tx_hash,
                        amount: u256_to_big_decimal(item.amount),
                        transaction_from_public_key: item.from,
                        asset: asset.clone(),
                        created_at: item.created_at,
                    }),
                    None => None,
                },
            )
            .collect::<Vec<_>>();

        Deposit::insert_bulk_deposits(pool, new_deposits).await?;

        next_token = Some(erc20_transfer_response.link.next_token);

        // Blastscan minimum req/s is 1, so we sleep for 1s after each request. probably fine for now since we won't be hitting
        // 100 eth/erc20 deposits/s for a while. Or probably ever.
        // TODO: Exponential backoff instead of sleep
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new();

    let config = config::Config::new_from_env();

    let mut pool = PgPool::connect(&config.database_url).await?;

    // get all eth transfers
    fetch_new_eth_deposits(&mut pool, &client).await?;

    // get all erc20 transfers
    fetch_new_erc20_deposits(&mut pool, &client).await?;

    Ok(())
}
