mod deposit;
mod send;
mod start;
mod types;
mod utils;

use std::sync::Arc;

use config::Config;
use deposit::{
    click_deposit_address_or_deposit_amount, receive_deposit_address, receive_deposit_amount,
    receive_deposit_coin_by_address, receive_deposit_coin_by_amount, receive_deposit_type,
};
use ethers::providers::{Http, Provider};
use log::info;
use price::PriceClient;
use sqlx::PgPool;
use start::{cancel, help, invalid_state, start};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
};
use tokio::sync::Mutex;
use types::{Command, State};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting bot...");

    let config = Config::new_from_env();
    let bot = Bot::new(config.teloxide_token.clone());

    // let pool = Arc::new(Mutex::new(
    //     PgPool::connect(&config.database_url).await.unwrap(),
    // ));

    let pool = PgPool::connect(&config.database_url).await.unwrap();

    let price_client = Arc::new(Mutex::new(
        PriceClient::new(Some(config.clone()), None).await.unwrap(),
    ));

    let mainnet_provider = Arc::new(Mutex::new(
        Provider::<Http>::try_from(config.mainnet_http_rpc_url).unwrap(),
    ));

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            mainnet_provider,
            pool,
            price_client,
            InMemStorage::<State>::new()
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<anyhow::Error> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Start].endpoint(start)) // .branch(case![Command::Cancel].endpoint(cancel)),
                .branch(case![Command::Deposit].endpoint(click_deposit_address_or_deposit_amount)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::UserClickedDeposit].endpoint(receive_deposit_type))
        .branch(
            case![State::UserInputtedDepositAddress { user_address }]
                .endpoint(receive_deposit_coin_by_address),
        )
        .branch(
            case![State::UserInputtedDepositAmount { deposit_amount }]
                .endpoint(receive_deposit_coin_by_amount),
        );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::AwaitingDepositAddress].endpoint(receive_deposit_address))
        .branch(case![State::AwaitingDepositAmount].endpoint(receive_deposit_amount))
        .branch(dptree::endpoint(invalid_state));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}
