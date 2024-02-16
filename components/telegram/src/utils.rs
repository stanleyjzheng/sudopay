use std::str::FromStr;

use db::balances::ANNUAL_YIELD_RATE;
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
// 0.000424
pub static APPROX_ETH_TRANSACTION_COST: u64 = 424000000000000_u64;

pub(crate) async fn get_user_username(bot: Bot, user_id: i64) -> Option<String> {
    match bot.get_chat(ChatId(user_id)).send().await {
        Ok(chat) => match chat.kind {
            ChatKind::Private(ChatPrivate { ref username, .. }) => username.clone(),
            _ => None,
        },
        Err(_) => None,
    }
}

fn calculate_daily_yield_with_bigdecimal(yearly_yield: &BigDecimal) -> BigDecimal {
    let one = BigDecimal::from(1);

    // this is really bad precision-wise, but since it's just for the UI and we're showing whole numbers, it's fine
    let yearly_yield_f64 = yearly_yield.to_f64().unwrap_or(0.0);

    let daily_yield_f64 = yearly_yield_f64.powf(1.0 / 365.0) - 1.0;

    BigDecimal::from_str(&daily_yield_f64.to_string()).unwrap_or(one)
}

fn next_free_tx(eth_balance: BigDecimal) -> u64 {
    let daily_yield = calculate_daily_yield_with_bigdecimal(&ANNUAL_YIELD_RATE);

    let daily_eth_yield = eth_balance * daily_yield;

    let daily_eth_yield = daily_eth_yield.to_u64().unwrap_or(1);

    if daily_eth_yield == 0 {
        return u64::MAX;
    }

    (APPROX_ETH_TRANSACTION_COST / daily_eth_yield) as u64
}

pub fn formatted_next_free_tx(eth_balance: BigDecimal) -> String {
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
