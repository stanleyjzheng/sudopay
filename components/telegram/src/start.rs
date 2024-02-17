use std::sync::Arc;

use crate::{
    types::{Command, MyDialogue},
    utils::formatted_next_free_tx,
};
use common::{
    types::SudoPayAsset,
    utils::{get_unit_amount, make_telegram_markdown_parser_happy},
};
use db::{balances::Balance, users::User};
use num_traits::ToPrimitive;
use price::{Asset, PriceClient};
use sqlx::{types::BigDecimal, PgPool};
use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use tokio::sync::Mutex;

use crate::utils::get_user_username;

static APPROX_ETH_TRANSACTION_COST: u64 = 424000000000000;

pub(crate) async fn start(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    price_client: Arc<Mutex<PriceClient>>,
    pool: PgPool,
) -> anyhow::Result<()> {
    log::debug!("start invoked");

    let price_client = price_client.lock().await;

    let eth_price = price_client.get_cached_price(Asset::Eth).await?;

    let user = User::get_user(&pool, dialogue.chat_id().0).await?;

    match user {
        Some(user) => {
            let user_public_key = user.seed_phrase_public_key;

            let balances = Balance::get_by_seed_phrase_public_key(&pool, &user_public_key).await?;

            let usdb_balance = get_unit_amount(&SudoPayAsset::Usdb, balances.usdb_balance);
            let eth_balance = get_unit_amount(&SudoPayAsset::Eth, balances.eth_balance);

            let next_free_tx = formatted_next_free_tx(&eth_balance);

            bot.send_message(
                msg.chat.id,
                make_telegram_markdown_parser_happy(format!(
                    "**Eth**: ${} \n🤑 SudoPay 📲 \nTWITTER_LINK_HERE \n\n═══ Your Balances ═══\n {} USDB\n {} ETH\n\nYou have {} free transactions, and an additional one coming... {}",
                    eth_price,
                    usdb_balance,
                    eth_balance,
                    (balances.accrued_yield_balance / BigDecimal::from(APPROX_ETH_TRANSACTION_COST)).to_i16().unwrap_or(0),
                    next_free_tx
                ))
            )
            .parse_mode(ParseMode::MarkdownV2)
            .disable_web_page_preview(true)
            .await?;
        }
        None => {
            log::debug!("didn't find a user");

            let telegram_tag = match get_user_username(bot.clone(), dialogue.chat_id().0).await {
                Some(username) => username,
                None => "Unknown".to_string(),
            };

            match User::get_user_by_telegram_tag(&pool, telegram_tag.clone()).await? {
                Some(user) => {
                    let user_public_key = user.seed_phrase_public_key;

                    let balances =
                        Balance::get_by_seed_phrase_public_key(&pool, &user_public_key).await?;

                    let usdb_balance = get_unit_amount(&SudoPayAsset::Usdb, balances.usdb_balance);
                    let eth_balance = get_unit_amount(&SudoPayAsset::Eth, balances.eth_balance);

                    let next_free_tx = formatted_next_free_tx(&eth_balance);

                    bot.send_message(
                        msg.chat.id,
                        make_telegram_markdown_parser_happy(format!(
                                "Welcome to SudoPay! You've already been sent a payment before you registered (click 'list transactions' below to find out from whom). \n**Eth**: ${} \n🤑 SudoPay 📲 \nTWITTER_LINK_HERE \n\n═══ Your Balances ═══\n {} USDB\n {} ETH\n\nYou have {} free transactions, and an additional one coming... {}",
                                eth_price,
                                eth_balance,
                                usdb_balance,
                                (balances.accrued_yield_balance / BigDecimal::from(APPROX_ETH_TRANSACTION_COST)).to_i16().unwrap_or(0),
                                next_free_tx
                            )),
                        )
                    .parse_mode(ParseMode::MarkdownV2)
                    .disable_web_page_preview(true)
                    .await?;
                }
                None => {
                    log::debug!("Didn't get a user via tag");

                    let user = User::new(&pool, dialogue.chat_id().0, telegram_tag, true).await?;
                    Balance::new(&pool, user.seed_phrase_public_key).await?;

                    bot.send_message(
                        msg.chat.id,
                        make_telegram_markdown_parser_happy("Welcome to SudoPay\\! \nYou have been registered. \n\n🤑 SudoPay 📲 \nTWITTER_LINK_HERE \n\n═══ Your Balances ═══\n 0 USDB\n 0 ETH".to_owned()),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .disable_web_page_preview(true)
                    .await?;
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
