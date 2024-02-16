use std::sync::Arc;

use crate::{
    types::{Command, MyDialogue},
    utils::{next_free_tx, APPROX_ETH_TRANSACTION_COST},
};
use db::{balances::Balance, users::User};
use price::{Asset, PriceClient};
use sqlx::{types::BigDecimal, PgPool};
use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use tokio::sync::Mutex;

use crate::utils::get_user_username;

pub(crate) async fn start(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    price_client: Arc<Mutex<PriceClient>>,
    pool: &PgPool,
) -> anyhow::Result<()> {
    let price_client = price_client.lock().await;

    let eth_price = price_client.get_cached_price(Asset::Eth).await?;

    let user = User::get_user(pool, dialogue.chat_id().0).await?;

    match user {
        Some(user) => {
            let user_public_key = user.seed_phrase_public_key;

            let balances = Balance::get_by_seed_phrase_public_key(pool, &user_public_key).await?;
            let next_free_tx = next_free_tx(balances.eth_balance.clone());

            bot.send_message(
                msg.chat.id,
                format!(
                    "**Eth**: ${} \n🤑 SudoPay 📲 [twitter](https://x.com/sudolabel) \n═══ Your Balances ═══\n {} USDB\n {} ETH\n\nYou have {} free transactions, and an additional one coming in {} days",
                    eth_price, 
                    balances.eth_balance, 
                    balances.usdb_balance, 
                    balances.accrued_yield_balance / BigDecimal::from(APPROX_ETH_TRANSACTION_COST), 
                    next_free_tx
                ),
            ).parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        None => {
            let telegram_tag = match get_user_username(bot.clone(), dialogue.chat_id().0).await {
                Some(username) => username,
                None => "Unknown".to_string(),
            };

            match User::get_user_by_telegram_tag(pool, telegram_tag.clone()).await? {
                Some(user) => {
                    let user_public_key = user.seed_phrase_public_key;

                    let balances = Balance::get_by_seed_phrase_public_key(pool, &user_public_key).await?;
                    let next_free_tx = next_free_tx(balances.eth_balance.clone());
        
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "Welcome to SudoPay. You've already been sent a payment before you registered (click 'list transactions' below to find out from whom). \n**Eth**: ${} \n🤑 SudoPay 📲 [twitter](https://x.com/sudolabel) \n═══ Your Balances ═══\n {} USDB\n {} ETH\n\nYou have {} free transactions, and an additional one coming in {} days",
                            eth_price, 
                            balances.eth_balance, 
                            balances.usdb_balance, 
                            balances.accrued_yield_balance / BigDecimal::from(APPROX_ETH_TRANSACTION_COST), 
                            next_free_tx
                        ),
                    ).parse_mode(ParseMode::MarkdownV2)
                    .await?;
                }
                None => {
                    User::new(pool, dialogue.chat_id().0, telegram_tag, true).await?;

                    bot.send_message(
                        msg.chat.id,
                        "Welcome to SudoPay! You have been registered. \n🤑 SudoPay 📲 [twitter](https://x.com/sudolabel) \n═══ Your Balances ═══\n 0 USDB\n 0 ETH",
                    ).parse_mode(ParseMode::MarkdownV2).await?;
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn help(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

pub(crate) async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Cancelling the dialogue.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

pub(crate) async fn invalid_state(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
    Ok(())
}
