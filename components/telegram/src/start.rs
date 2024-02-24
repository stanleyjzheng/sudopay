use std::sync::Arc;

use crate::{
    deposit::click_deposit_address_or_deposit_amount,
    send::input_send_address,
    types::{Command, MyDialogue, State},
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
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

use crate::utils::get_user_username;

static APPROX_ETH_TRANSACTION_COST: u64 = 424000000000000;

pub(crate) async fn start(
    bot: Bot,
    dialogue: MyDialogue,
    price_client: Arc<Mutex<PriceClient>>,
    pool: PgPool,
) -> anyhow::Result<()> {
    log::debug!("start invoked");

    let menu_actions = InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback("Send a transaction", "send_transaction"),
            InlineKeyboardButton::callback("Send an invoice", "send_invoice"),
        ],
        vec![
            InlineKeyboardButton::callback("Schedule a transaction", "schedule_transaction"),
            InlineKeyboardButton::callback("Deposit assets", "deposit_assets"),
        ],
        vec![
            InlineKeyboardButton::callback("List deposits", "list_deposits"),
            InlineKeyboardButton::callback("List transactions", "list_transactions"),
        ],
        vec![InlineKeyboardButton::callback("Settings", "settings")],
    ]);

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
                dialogue.chat_id(),
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
            .reply_markup(menu_actions)
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
                    User::set_telegram_id(&pool, telegram_tag, dialogue.chat_id().0).await?;
                    User::set_onboarded(&pool, dialogue.chat_id().0).await?;

                    let user_public_key = user.seed_phrase_public_key;

                    let balances =
                        Balance::get_by_seed_phrase_public_key(&pool, &user_public_key).await?;

                    let usdb_balance = get_unit_amount(&SudoPayAsset::Usdb, balances.usdb_balance);
                    let eth_balance = get_unit_amount(&SudoPayAsset::Eth, balances.eth_balance);

                    let next_free_tx = formatted_next_free_tx(&eth_balance);

                    bot.send_message(
                        dialogue.chat_id(),
                        make_telegram_markdown_parser_happy(format!(
                                "Welcome to SudoPay! You've already been sent a payment before you registered (click 'list transactions' below to find out from whom). \n\n**Eth**: ${} \n🤑 SudoPay 📲 \nTWITTER_LINK_HERE \n\n═══ Your Balances ═══\n {} USDB\n {} ETH\n\nYou have {} free transactions, and an additional one coming... {}",
                                eth_price,
                                usdb_balance,
                                eth_balance,
                                (balances.accrued_yield_balance / BigDecimal::from(APPROX_ETH_TRANSACTION_COST)).to_i16().unwrap_or(0),
                                next_free_tx
                            )),
                        )
                    .parse_mode(ParseMode::MarkdownV2)
                    .disable_web_page_preview(true)
                    .reply_markup(menu_actions)
                    .await?;
                }
                None => {
                    log::debug!("Didn't get a user via tag");

                    let user = User::new(&pool, dialogue.chat_id().0, telegram_tag, true).await?;
                    Balance::new(&pool, user.seed_phrase_public_key).await?;

                    bot.send_message(
                        dialogue.chat_id(),
                        make_telegram_markdown_parser_happy("Welcome to SudoPay\\! \nYou have been registered. \n\n🤑 SudoPay 📲 \nTWITTER_LINK_HERE \n\n═══ Your Balances ═══\n 0 USDB\n 0 ETH".to_owned()),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .disable_web_page_preview(true)
                    .reply_markup(menu_actions)
                    .await?;
                }
            }
        }
    }

    dialogue.update(State::UserClickedMenu).await?;

    Ok(())
}

async fn list_transactions(
    bot: Bot,
    dialogue: MyDialogue,
    price_client: Arc<Mutex<PriceClient>>,
    pool: PgPool,
) -> anyhow::Result<()> {
    bot.send_message(
        dialogue.chat_id(),
        "One transaction found:\n@fiveoutofnine sent you 200 USDC 18 minutes ago",
    )
    .await?;
    dialogue.exit().await?;

    start(bot, dialogue, price_client, pool).await?;

    Ok(())
}

async fn settings(bot: Bot, dialogue: MyDialogue) -> anyhow::Result<()> {
    let menu_actions = InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback("Set a password", "send_transaction"),
            InlineKeyboardButton::callback("Export recovery phrase", "send_invoice"),
        ],
        vec![InlineKeyboardButton::callback(
            "Set auto withdrawal threshold",
            "schedule_transaction",
        )],
        vec![InlineKeyboardButton::callback(
            "Sponsor gas fees for transactions to you",
            "deposit_assets",
        )],
        vec![InlineKeyboardButton::callback(
            "Return to main menu",
            "settings",
        )],
    ]);

    bot.send_message(dialogue.chat_id(), "What action would you like to perform?")
        .reply_markup(menu_actions)
        .await?;
    dialogue.exit().await?;

    Ok(())
}

pub(crate) async fn handle_menu_click(
    bot: Bot,
    dialogue: MyDialogue,
    price_client: Arc<Mutex<PriceClient>>,
    pool: PgPool,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    match &q.data {
        Some(data) => match data.as_str() {
            "send_transaction" => input_send_address(bot, dialogue).await?,
            "list_transactions" => list_transactions(bot, dialogue, price_client, pool).await?,
            "deposit_assets" => click_deposit_address_or_deposit_amount(bot, dialogue).await?,
            "settings" => settings(bot, dialogue).await?,
            _ => {
                bot.send_message(dialogue.chat_id(), "Invalid action")
                    .await?;
                dialogue.exit().await?
            }
        },
        None => {
            bot.send_message(dialogue.chat_id(), "Invalid action")
                .await?;
            dialogue.exit().await?
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
