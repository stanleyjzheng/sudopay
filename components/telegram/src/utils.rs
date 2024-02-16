use std::str::FromStr;

use db::balances::ANNUAL_YIELD_RATE;
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

pub(crate) fn calculate_daily_yield_with_bigdecimal(yearly_yield: &BigDecimal) -> BigDecimal {
    let one = BigDecimal::from(1);

    // this is really bad precision-wise, but since it's just for the UI and we're showing whole numbers, it's fine
    let yearly_yield_f64 = yearly_yield.to_f64().unwrap_or(0.0);

    let daily_yield_f64 = yearly_yield_f64.powf(1.0 / 365.0) - 1.0;

    BigDecimal::from_str(&daily_yield_f64.to_string()).unwrap_or(one)
}

pub(crate) fn next_free_tx(eth_balance: BigDecimal) -> u64 {
    let daily_yield = calculate_daily_yield_with_bigdecimal(&ANNUAL_YIELD_RATE);

    let daily_eth_yield = eth_balance * daily_yield;
    (APPROX_ETH_TRANSACTION_COST / daily_eth_yield.to_u64().unwrap_or(1)) as u64
}
