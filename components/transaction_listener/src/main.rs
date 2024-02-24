mod blastscan;
mod notifications;

use chrono::Utc;
use common::{
    types::SudoPayAsset,
    utils::{get_unit_amount, u256_to_big_decimal, TOKEN_ADDRESS_TO_ASSET},
};
use config::Config;
use db::{
    balances::Balance,
    deposits::{Deposit, DepositRequest, NewDeposit},
    users::User,
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use reqwest::Client;
use sqlx::PgPool;
use teloxide::Bot;
use tokio::time::{sleep, Duration};

use crate::blastscan::{list_erc20_transfers, list_eth_transfers};
use crate::notifications::notify_of_deposit;

static DEPOSIT_REQUEST_DURATION_SECONDS: i64 = 180;
// static DEPOSIT_REQUEST_DURATION_SECONDS: i64 = 86400 * 5;
static DEPOSIT_ANTI_FRONTRUN_DURATION_SECONDS: i64 = 3;

// TODO: CEX withdrawal addresses upon mainnet, or an automated process to pull wallet labels
static CEX_WITHDRAWAL_ADDRESSES: Lazy<Vec<String>> =
    Lazy::new(|| vec!["0xc0ffeebabe000000000000000000000000000000".to_string()]);

// returns bool whether to early exit
async fn commit_deposit_to_user_balance(
    pool: &PgPool,
    deposit_requests: Vec<DepositRequest>,
    new_deposit: &NewDeposit,
    teloxide_bot: &Bot,
) -> anyhow::Result<bool> {
    let non_cex_deposit_addresses = deposit_requests
        .into_iter()
        .filter(|deposit| {
            !CEX_WITHDRAWAL_ADDRESSES.contains(&deposit.from_address.clone().unwrap_or_default())
        })
        .collect::<Vec<DepositRequest>>();

    let non_filled_deposit_requests = non_cex_deposit_addresses
        .iter()
        .filter(|deposit| deposit.matched_transaction_id.is_none())
        .collect::<Vec<&DepositRequest>>();

    let unique_depositing_addresses = non_filled_deposit_requests
        .iter()
        .map(|deposit| deposit.depositor_public_key.clone())
        .unique()
        .collect::<Vec<String>>();

    match unique_depositing_addresses.len() {
        0 => {
            // didn't match a deposit, proceed with amount matching below
        }
        1 => {
            // we matched a deposit, exit and set the deposit matched
            Deposit::set_deposit_matched(pool, &new_deposit.transaction_id.clone()).await?;
            DepositRequest::set_matched_transaction_id(
                pool,
                non_cex_deposit_addresses[0].id,
                new_deposit.transaction_id.clone(),
            )
            .await?;
            Balance::add_to_balance(
                pool,
                &non_cex_deposit_addresses[0].depositor_public_key,
                new_deposit.amount.clone(),
                &non_cex_deposit_addresses[0].asset,
            )
            .await?;

            let user_id = User::get_user_by_address(
                pool,
                non_cex_deposit_addresses[0].depositor_public_key.clone(),
            )
            .await?
            .map(|user| user.telegram_id)
            .unwrap_or_default();

            let unit_amount = get_unit_amount(&new_deposit.asset, new_deposit.amount.clone());

            notify_of_deposit(
                teloxide_bot,
                user_id,
                unit_amount,
                new_deposit.asset.clone(),
            )
            .await?;

            return Ok(true);
        }
        _ => {
            // we matched multiple deposits, early exit and error for support to handle
            log::error!(
                "Multiple deposit requests matched a single deposit: {:?}",
                unique_depositing_addresses
            );
            return Ok(true);
        }
    };

    Ok(false)
}

async fn match_deposits_to_user_deposit_requests(
    pool: &PgPool,
    teloxide_bot: &Bot,
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
        new_deposit.transaction_from_public_key.to_lowercase(),
        new_deposit.asset.clone(),
        start_time,
        end_time,
    )
    .await?;

    dbg!(deposit_addresses.clone());

    let early_exit =
        commit_deposit_to_user_balance(pool, deposit_addresses, &new_deposit, teloxide_bot).await?;

    if early_exit {
        return Ok(());
    }

    let deposit_requests = DepositRequest::from_amount_by_time(
        pool,
        new_deposit.amount.clone(),
        new_deposit.asset.clone(),
        start_time,
        end_time,
    )
    .await?;
    commit_deposit_to_user_balance(pool, deposit_requests, &new_deposit, teloxide_bot).await?;

    Ok(())
}

async fn fetch_new_eth_deposits(
    pool: &mut PgPool,
    client: &Client,
    config: &Config,
    teloxide_bot: &Bot,
) -> anyhow::Result<()> {
    let mut next_token: Option<String> = None;
    let mut all_eth_transfer_ids: Vec<String> = Vec::new();

    loop {
        let eth_transfer_response = list_eth_transfers(client, next_token.clone(), config).await?;

        // Blastscan minimum req/s limit is 1, so we sleep for 500ms after each request. probably fine for now since we won't be hitting
        // 100 eth/erc20 deposits/s for a while. Or probably ever.
        // TODO: Exponential backoff instead of sleep
        sleep(Duration::from_millis(1000)).await;

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
                transaction_id: item.id.to_lowercase(),
                amount: u256_to_big_decimal(item.value),
                transaction_from_public_key: item.from.to_lowercase(),
                asset: SudoPayAsset::Eth,
                created_at: item.timestamp,
            })
            .collect::<Vec<_>>();

        dbg!(new_deposits.clone());

        Deposit::insert_bulk_deposits(pool, new_deposits.clone()).await?;

        for new_deposit in new_deposits {
            match_deposits_to_user_deposit_requests(pool, teloxide_bot, new_deposit).await?;
        }

        match eth_transfer_response.link.next_token {
            Some(token) => {
                next_token = Some(token);
            }
            None => {
                break;
            }
        }

        // Blastscan minimum req/s is 1, so we sleep for 1s after each request. probably fine for now since we won't be hitting
        // 100 eth/erc20 deposits/s for a while. Or probably ever.
        // TODO: Exponential backoff instead of sleep
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

async fn fetch_new_erc20_deposits(
    pool: &mut PgPool,
    client: &Client,
    config: &Config,
    teloxide_bot: &Bot,
) -> anyhow::Result<()> {
    let mut next_token: Option<String> = None;
    let mut all_erc20_transfer_ids: Vec<String> = Vec::new();

    loop {
        let erc20_transfer_response =
            list_erc20_transfers(client, next_token.clone(), config).await?;

        // Blastscan minimum req/s limit is 1, so we sleep for 500ms after each request. probably fine for now since we won't be hitting
        // 100 eth/erc20 deposits/s for a while. Or probably ever.
        // TODO: Exponential backoff instead of sleep
        sleep(Duration::from_millis(1000)).await;

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
                        transaction_id: item.tx_hash.to_lowercase(),
                        amount: u256_to_big_decimal(item.amount),
                        transaction_from_public_key: item.from.to_ascii_lowercase(),
                        asset: asset.clone(),
                        created_at: item.created_at,
                    }),
                    None => None,
                },
            )
            .collect::<Vec<_>>();

        dbg!(new_deposits.clone());

        Deposit::insert_bulk_deposits(pool, new_deposits.clone()).await?;

        for new_deposit in new_deposits {
            match_deposits_to_user_deposit_requests(pool, teloxide_bot, new_deposit).await?;
        }

        match erc20_transfer_response.link.next_token {
            Some(token) => {
                next_token = Some(token);
            }
            None => {
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    log::info!("Starting transaction listener...");

    let client = Client::new();

    let config = config::Config::new_from_env();

    let mut pool = PgPool::connect(&config.database_url).await?;

    let teloxide_bot = Bot::new(config.teloxide_token.clone());

    loop {
        // get all eth transfers
        fetch_new_eth_deposits(&mut pool, &client, &config, &teloxide_bot).await?;

        // get all erc20 transfers
        fetch_new_erc20_deposits(&mut pool, &client, &config, &teloxide_bot).await?;
    }
}
