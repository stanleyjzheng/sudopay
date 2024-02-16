mod deposit;
mod types;

use std::sync::Arc;

use config::Config;
use deposit::{
    click_deposit_address_or_deposit_amount, receive_deposit_address, receive_deposit_amount,
    receive_deposit_coin_by_address, receive_deposit_coin_by_amount, receive_deposit_type,
};
use log::info;
use price::{Asset, PriceClient};
use sqlx::PgPool;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use tokio::sync::Mutex;
use types::{Command, MyDialogue, State};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting bot...");

    let config = Config::new_from_env();
    let bot = Bot::new(config.teloxide_token.clone());

    let pool = PgPool::connect(&config.database_url).await.unwrap();
    let price_client = Arc::new(Mutex::new(
        PriceClient::new(Some(config), Some(pool.clone()))
            .await
            .unwrap(),
    ));

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
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

async fn start(
    bot: Bot,
    _dialogue: MyDialogue,
    msg: Message,
    price_client: Arc<Mutex<PriceClient>>,
) -> anyhow::Result<()> {
    let price_client = price_client.lock().await;

    let eth_price = price_client.get_cached_price(Asset::Eth).await?;
    // TODO: Fetch USDB balance
    let usdb_balance = 1000;
    // TODO: Fetch ETH balance
    let eth_balance = 0.5;
    bot.send_message(
        msg.chat.id,
        format!(
            "**Eth**: ${} \n🤑 SudoPay 📲 [twitter](https://x.com/sudolabel) \n═══ Your Balances ═══\n {} USDB\n {} ETH",
            eth_price, usdb_balance, eth_balance
        ),
    )
    .await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Cancelling the dialogue.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
    Ok(())
}
