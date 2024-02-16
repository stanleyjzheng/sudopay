use common::types::SudoPayAsset;
use sqlx::types::BigDecimal;
use teloxide::{
    requests::{Request, Requester},
    types::ChatId,
    Bot,
};

async fn send_message_to_user(bot: &Bot, user_id: i64, message: &str) -> anyhow::Result<()> {
    let chat_id = ChatId(user_id);
    bot.send_message(chat_id, message).send().await?;

    Ok(())
}

pub(crate) async fn notify_of_deposit(
    bot: &Bot,
    user_id: i64,
    deposit_unit_amount: BigDecimal,
    asset: SudoPayAsset,
) -> anyhow::Result<()> {
    let message = format!(
        "You have received a deposit of {} {}",
        deposit_unit_amount, asset
    );
    send_message_to_user(bot, user_id, &message).await?;

    Ok(())
}
