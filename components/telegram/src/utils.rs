use ethers::{
    providers::{Http, Middleware, Provider},
    types::H160,
};
use num_traits::cast::ToPrimitive;
use sqlx::types::BigDecimal;
use teloxide::{
    prelude::Bot,
    requests::{Request, Requester},
    types::{ChatId, ChatKind, ChatPrivate},
};

// TODO: calculate based on gas price/limit
pub static APPROX_ETH_TRANSACTION_COST: f64 = 0.000424_f64;
pub static DAILY_YIELD: f64 = 0.000094_f64;

pub(crate) async fn get_user_username(bot: Bot, user_id: i64) -> Option<String> {
    match bot.get_chat(ChatId(user_id)).send().await {
        Ok(chat) => match chat.kind {
            ChatKind::Private(ChatPrivate { ref username, .. }) => username.clone(),
            _ => None,
        },
        Err(_) => None,
    }
}

fn next_free_tx(eth_balance: &BigDecimal) -> u64 {
    let daily_eth_yield = eth_balance.to_f64().unwrap_or(0_f64) * DAILY_YIELD;

    if daily_eth_yield <= 0.000001_f64 {
        return u64::MAX;
    }

    (APPROX_ETH_TRANSACTION_COST / daily_eth_yield) as u64
}

pub fn formatted_next_free_tx(eth_balance: &BigDecimal) -> String {
    let next_free_tx = next_free_tx(eth_balance);
    match next_free_tx {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        u64::MAX => "never (deposit some ETH!)".to_string(),
        _ => format!("in {} days", next_free_tx),
    }
}

pub(crate) async fn ens_to_address(
    provider: Provider<Http>,
    ens_name: String,
) -> anyhow::Result<H160> {
    provider
        .resolve_name(&ens_name)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}
