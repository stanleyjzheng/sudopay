use std::str::FromStr;

use crate::types::{MyDialogue, State};

use common::types::SudoPayAsset;
use db::{deposits::DepositRequest, users::User};
use ethers::types::H160;
use sqlx::{types::BigDecimal, PgPool};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

pub async fn click_deposit_address_or_deposit_amount(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> anyhow::Result<()> {
    let deposit_methods = ["Specify deposit address", "Specify deposit amount"]
        .map(|product| InlineKeyboardButton::callback(product, product));

    bot.send_message(msg.chat.id, "Would you like to specify the address you are depositing from (more secure, if you are depositing from a known wallet), or the exact amount of your deposit (if you are depositing from an exchange).")
        .reply_markup(InlineKeyboardMarkup::new([deposit_methods]))
        .await?;

    dialogue.update(State::UserClickedDeposit).await?;

    Ok(())
}

pub async fn click_choose_deposit_coin(bot: Bot, dialogue: MyDialogue) -> anyhow::Result<()> {
    let deposit_assets =
        ["USDB", "ETH", "WETH"].map(|product| InlineKeyboardButton::callback(product, product));

    bot.send_message(
        dialogue.chat_id(),
        "What asset will you be depositing? Any other tokens sent to this address will be lost.",
    )
    .reply_markup(InlineKeyboardMarkup::new([deposit_assets]))
    .await?;

    Ok(())
}

pub async fn input_deposit_address(bot: Bot, dialogue: MyDialogue) -> anyhow::Result<()> {
    bot.send_message(dialogue.chat_id(), "Please enter your deposit address.")
        .await?;

    dialogue.update(State::AwaitingDepositAddress).await?;

    Ok(())
}

pub async fn input_deposit_amount(bot: Bot, dialogue: MyDialogue) -> anyhow::Result<()> {
    bot.send_message(dialogue.chat_id(), "Please enter your deposit amount.")
        .await?;

    dialogue.update(State::AwaitingDepositAmount).await?;

    Ok(())
}

pub async fn receive_deposit_address(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> anyhow::Result<()> {
    match msg.text().map(ToOwned::to_owned) {
        Some(deposit_address) => {
            let deposit_address = match H160::from_str(&deposit_address) {
                Ok(deposit_address) => deposit_address,
                Err(_) => {
                    bot.send_message(msg.chat.id, "Could not parse deposit address; verify that it's a valid deposit address, then re-enter it.")
                        .await?;
                    return Ok(());
                }
            };

            dialogue
                .update(State::UserInputtedDepositAddress {
                    user_address: deposit_address,
                })
                .await?;

            click_choose_deposit_coin(bot, dialogue).await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Could not parse deposit address; verify that it's a valid deposit address, then re-enter it.")
                .await?;
        }
    }

    Ok(())
}

pub async fn receive_deposit_amount(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> anyhow::Result<()> {
    match msg.text().map(ToOwned::to_owned) {
        Some(deposit_amount) => {
            let deposit_amount = match BigDecimal::from_str(&deposit_amount) {
                Ok(deposit_amount) => deposit_amount,
                Err(_) => {
                    bot.send_message(msg.chat.id, "Could not parse amount; verify that it's a valid number, then re-enter it.")
                        .await?;
                    return Ok(());
                }
            };

            dialogue
                .update(State::UserInputtedDepositAmount { deposit_amount })
                .await?;

            click_choose_deposit_coin(bot, dialogue).await?;
        }
        None => {
            bot.send_message(
                msg.chat.id,
                "Could not parse amount; verify that it's a valid number, then re-enter it.",
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn receive_deposit_type(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    match &q.data {
        Some(deposit_type) => match deposit_type.as_str() {
            "Specify deposit address" => input_deposit_address(bot, dialogue).await?,
            "Specify deposit amount" => input_deposit_amount(bot, dialogue).await?,
            _ => {
                dialogue.exit().await?;
            }
        },
        None => dialogue.exit().await?,
    }

    Ok(())
}

pub async fn receive_deposit_coin_by_amount(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    amount: BigDecimal,
    pool: &PgPool,
) -> anyhow::Result<()> {
    match &q.data {
        Some(deposit_type) => match deposit_type.parse::<SudoPayAsset>() {
            Ok(asset) => {
                let user = match User::get_user(pool, dialogue.chat_id().0).await? {
                    Some(u) => u,
                    None => {
                        bot.send_message(
                            dialogue.chat_id(),
                            "You are not registered. Please register with /start.",
                        )
                        .await?;
                        dialogue.exit().await?;
                        return Ok(());
                    }
                };

                DepositRequest::new(
                    pool,
                    user.seed_phrase_public_key,
                    asset.clone(),
                    Some(amount.clone()),
                    None,
                )
                .await?;

                let return_to_main_menu = ["Return to main menu"]
                    .map(|product| InlineKeyboardButton::callback(product, product));

                bot.send_message(
                    dialogue.chat_id(),
                    format!("You have created a deposit request for {} {:?}. This deposit request expires in an hour; please create a new deposit request past this time. Be sure to deposit the exact amount specified.", amount, asset),
                ).reply_markup(InlineKeyboardMarkup::new([return_to_main_menu])).await?;
            }
            Err(_) => {
                dialogue.exit().await?;
            }
        },
        None => dialogue.exit().await?,
    }

    Ok(())
}

pub async fn receive_deposit_coin_by_address(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    address: H160,
    pool: &PgPool,
) -> anyhow::Result<()> {
    match &q.data {
        Some(deposit_type) => match deposit_type.parse::<SudoPayAsset>() {
            Ok(asset) => {
                let user = match User::get_user(pool, dialogue.chat_id().0).await? {
                    Some(u) => u,
                    None => {
                        bot.send_message(
                            dialogue.chat_id(),
                            "You are not registered. Please register with /start.",
                        )
                        .await?;
                        dialogue.exit().await?;
                        return Ok(());
                    }
                };

                DepositRequest::new(
                    pool,
                    user.seed_phrase_public_key,
                    asset.clone(),
                    None,
                    Some(format!("{:#?}", address)),
                )
                .await?;

                let return_to_main_menu = ["Return to main menu"]
                    .map(|product| InlineKeyboardButton::callback(product, product));

                bot.send_message(
                    dialogue.chat_id(),
                    format!("You have created a deposit request for {} from your address {:#?}. This deposit request expires in 3 minutes; please create a new deposit request past this time.", address, asset),
                ).reply_markup(InlineKeyboardMarkup::new([return_to_main_menu])).await?;
            }
            Err(_) => {
                dialogue.exit().await?;
            }
        },
        None => dialogue.exit().await?,
    }

    Ok(())
}
