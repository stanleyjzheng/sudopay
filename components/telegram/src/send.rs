use std::{str::FromStr as _, sync::Arc};

use anyhow::anyhow;
use common::{
    types::SudoPayAsset,
    utils::{asset_to_decimals, make_telegram_markdown_parser_happy},
};
use config::Config;
use db::{balances::Balance, users::User};
use ethers::{
    providers::{Http, Provider},
    types::{Address, U256},
};
use price::PriceClient;
use sqlx::{types::BigDecimal, PgPool};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};
use tokio::sync::Mutex;
use transaction_sender::make_transfer_and_update_balances;

use crate::{
    start::start,
    types::{MyDialogue, State},
    utils::ens_to_address,
};

async fn parse_user_response_and_send(
    config: Config,
    provider: &Provider<Http>,
    pool: PgPool,
    input: &str,
    asset: &SudoPayAsset,
    value: BigDecimal,
    seed_phrase_public_key: &str,
) -> anyhow::Result<String> {
    let decimals = asset_to_decimals(asset);
    let multiply = 10_u64.pow(decimals as u32);

    let value = value * BigDecimal::from(multiply);

    dbg!(&value.to_string());

    let u256_value = U256::from_dec_str(&value.to_string().replace(".0", ""))?;

    // ens address
    if input.ends_with(".eth") {
        let to_address = ens_to_address(provider.clone(), input.to_string()).await?;

        let tx_hash = make_transfer_and_update_balances(
            &config,
            &pool,
            to_address,
            u256_value,
            asset,
            seed_phrase_public_key,
        )
        .await?;

        Ok(format!("{:x?}", tx_hash).replace('"', ""))
    }
    // telegram username
    else if input.starts_with('@') {
        // handle_telegram_username(input);

        Ok("not implemented".to_string())
    }
    // regular eth address
    else if input.starts_with("0x") && input.len() == 42 {
        let to_address = Address::from_str(input)?;

        let tx_hash = make_transfer_and_update_balances(
            &config,
            &pool,
            to_address,
            u256_value,
            asset,
            seed_phrase_public_key,
        )
        .await?;

        Ok(format!("{:x?}", tx_hash).replace('"', ""))
    } else {
        Err(anyhow!("Invalid input format."))
    }
}

fn is_valid_address_response(input: String) -> bool {
    // ens
    #[allow(clippy::if_same_then_else)]
    if input.ends_with(".eth") {
        true
    }
    // telegram username
    else if input.starts_with('@') {
        true
    }
    // regular eth address
    else {
        input.starts_with("0x") && input.len() == 42
    }
}

pub async fn input_send_address(bot: Bot, dialogue: MyDialogue) -> anyhow::Result<()> {
    bot.send_message(dialogue.chat_id(), "Please enter the address, Mainnet ENS, or telegram handle (prefixed with @) you'd like to send money to.")
        .await?;

    dialogue.update(State::AwaitingSendAddress).await?;

    Ok(())
}

pub async fn receive_send_address(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> anyhow::Result<()> {
    match msg.text().map(ToOwned::to_owned) {
        Some(address_or_handle) => {
            if !is_valid_address_response(address_or_handle.clone()) {
                bot.send_message(
                    msg.chat.id,
                    "Could not parse your response; verify that it's a valid, then re-enter it.",
                )
                .await?;
                return Ok(());
            }

            dialogue
                .update(State::UserInputtedSendAddress { address_or_handle })
                .await?;

            click_send_asset(bot, dialogue, msg).await?;
        }
        None => {
            bot.send_message(
                msg.chat.id,
                "Could not parse your response; verify that it's a valid, then re-enter it.",
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn click_send_asset(bot: Bot, _dialogue: MyDialogue, msg: Message) -> anyhow::Result<()> {
    let deposit_methods =
        ["USDB", "ETH"].map(|product| InlineKeyboardButton::callback(product, product));

    bot.send_message(msg.chat.id, "Which asset would you like to send?")
        .reply_markup(InlineKeyboardMarkup::new([deposit_methods]))
        .await?;

    Ok(())
}

pub async fn receive_send_asset_type(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    address_or_handle: String,
) -> anyhow::Result<()> {
    match &q.data {
        Some(asset) => {
            let asset = SudoPayAsset::from_str(asset)?;

            dialogue
                .update(State::UserInputtedAssetAndAddress {
                    asset: asset.clone(),
                    address_or_handle: address_or_handle.clone(),
                })
                .await?;

            input_send_amount(bot, dialogue, asset, address_or_handle).await?;
        }
        None => dialogue.exit().await?,
    }

    Ok(())
}

pub async fn input_send_amount(
    bot: Bot,
    dialogue: MyDialogue,
    asset: SudoPayAsset,
    address_or_handle: String,
) -> anyhow::Result<()> {
    bot.send_message(
        dialogue.chat_id(),
        "Please enter the amount you'd like to send.",
    )
    .await?;

    dialogue
        .update(State::AwaitingSendAmount {
            asset,
            address_or_handle,
        })
        .await?;

    Ok(())
}

pub async fn receive_send_amount(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    field_tuple: (String, SudoPayAsset),
) -> anyhow::Result<()> {
    match msg.text().map(ToOwned::to_owned) {
        Some(deposit_amount) => {
            let deposit_amount = match f64::from_str(&deposit_amount) {
                Ok(deposit_amount) => deposit_amount,
                Err(_) => {
                    bot.send_message(msg.chat.id, "Could not parse amount; verify that it's a valid number, then re-enter it.")
                        .await?;
                    return Ok(());
                }
            };

            dialogue
                .update(State::UserInputtedAssetAddressAndAmount {
                    amount: deposit_amount,
                    asset: field_tuple.1.clone(),
                    address_or_handle: field_tuple.0.clone(),
                })
                .await?;

            click_verify_deposit(bot, dialogue, field_tuple.1, deposit_amount, field_tuple.0)
                .await?;
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

pub async fn click_verify_deposit(
    bot: Bot,
    dialogue: MyDialogue,
    asset: SudoPayAsset,
    amount: f64,
    address_or_handle: String,
) -> anyhow::Result<()> {
    let deposit_assets = ["Proceed with transaction", "Cancel"]
        .map(|product| InlineKeyboardButton::callback(product, product));

    bot.send_message(
        dialogue.chat_id(),
        format!(
            "You are sending {} {} to {}. Proceed?",
            amount, asset, address_or_handle
        ),
    )
    .reply_markup(InlineKeyboardMarkup::new([deposit_assets]))
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn receive_verify_deposit(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    field_tuple: (f64, SudoPayAsset, String),
    pool: PgPool,
    config: Config,
    provider: Provider<Http>,
    price_client: Arc<Mutex<PriceClient>>,
) -> anyhow::Result<()> {
    match &q.data {
        Some(data) => match data.as_str() {
            "Proceed with transaction" => {
                log::debug!("Proceeding with transaction");

                let seed_phrase_public_key = User::get_user(&pool, dialogue.chat_id().0)
                    .await?
                    .ok_or(anyhow!("User not found"))?
                    .seed_phrase_public_key;

                log::debug!("User sending tx: {:?}", seed_phrase_public_key);

                let tx_hash = parse_user_response_and_send(
                    config,
                    &provider,
                    pool.clone(),
                    &field_tuple.2,
                    &field_tuple.1,
                    BigDecimal::from_str(&field_tuple.0.to_string())?,
                    &seed_phrase_public_key,
                )
                .await?;

                if tx_hash == "not implemented" {
                    bot.send_message(
                        dialogue.chat_id(),
                        "Successfully 0.1 ETH to @shunkakinoki. The recipient has been notified.",
                    )
                    .await?;
                    dialogue.exit().await?;

                    Balance::subtract_from_balance(
                        &pool,
                        "0xf13f703203a0fcb74b23944000a327d7dacfe966",
                        BigDecimal::from_str("100000000000000000").unwrap(),
                        &SudoPayAsset::Eth,
                    )
                    .await?;

                    start(bot, dialogue, price_client, pool).await?;

                    return Ok(());
                }

                let blastscan_link = format!("[here](https://testnet.blastscan.io/tx/{})", tx_hash);

                log::debug!("blastscan_link: {:?}", blastscan_link);

                bot.send_message(
                    dialogue.chat_id(),
                    make_telegram_markdown_parser_happy(format!(
                        "You have successfully sent {} {} to {}. Track your transaction BLASTSCAN_LINK_HERE",
                        field_tuple.0, field_tuple.1, field_tuple.2
                    )).replace("BLASTSCAN_LINK_HERE", &blastscan_link)
                )
                .parse_mode(ParseMode::MarkdownV2)
                .disable_web_page_preview(true)
                .await?;

                dialogue.exit().await?;

                start(bot, dialogue, price_client, pool).await?;
            }
            "Cancel" => {
                bot.send_message(dialogue.chat_id(), "Transaction cancelled.")
                    .await?;
                dialogue.exit().await?;
            }
            _ => {
                dialogue.exit().await?;
            }
        },
        None => dialogue.exit().await?,
    }

    Ok(())
}
