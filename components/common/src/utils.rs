use std::{collections::HashMap, str::FromStr};

use chrono::{DateTime, Utc};
use ethers::types::{H160, U256};
use once_cell::sync::Lazy;
use serde::{de, Deserialize, Deserializer};
use serde_json::Value;
use sqlx::types::BigDecimal;

use crate::types::SudoPayAsset;

pub static TOKEN_ADDRESS_TO_ASSET: Lazy<HashMap<String, SudoPayAsset>> = Lazy::new(|| {
    HashMap::from([
        (
            "0x4200000000000000000000000000000000000023".to_string(),
            SudoPayAsset::Weth,
        ),
        (
            "0x4200000000000000000000000000000000000022".to_string(),
            SudoPayAsset::Usdb,
        ),
    ])
});

pub fn asset_to_decimals(asset: &SudoPayAsset) -> u64 {
    match asset.to_owned() {
        SudoPayAsset::Weth => 18,
        SudoPayAsset::Usdb => 18,
        SudoPayAsset::Eth => 18,
    }
}

pub fn asset_to_address(asset: &SudoPayAsset) -> Option<H160> {
    match asset.to_owned() {
        SudoPayAsset::Weth => {
            Some(H160::from_str("0x4200000000000000000000000000000000000023").unwrap())
        }
        SudoPayAsset::Usdb => {
            Some(H160::from_str("0x4200000000000000000000000000000000000022").unwrap())
        }
        SudoPayAsset::Eth => None,
    }
}

pub fn deserialize_u256_from_json_number_or_string<'de, D>(
    deserializer: D,
) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<Value> = Deserialize::deserialize(deserializer)?;
    if let Some(value) = value {
        match value {
            Value::String(s) => {
                let radix = if s.starts_with("0x") { 16 } else { 10 };
                Ok(U256::from_str_radix(&s, radix).map_err(de::Error::custom)?)
            }
            Value::Number(n) => Ok(U256::from_dec_str(&n.to_string()).map_err(de::Error::custom)?),
            _ => Err(de::Error::custom("Expected string or number")),
        }
    } else {
        Err(de::Error::custom("Expected string or number"))
    }
}

pub fn deserialize_iso8601_date_time<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)
}

pub fn u256_to_big_decimal(value: U256) -> BigDecimal {
    let value_str = value.to_string();

    // theoretically this should never panic since U256 < BigDecimal::Max()
    BigDecimal::from_str(&value_str).unwrap()
}

pub fn make_telegram_markdown_parser_happy(message: String) -> String {
    // telegram's markdown parser is the bane of my existence.

    message
        .replace('.', "\\.")
        .replace(')', "\\)")
        .replace('(', "\\(")
        .replace('!', "\\!")
        // we escape the brackets we need for the link, so we replace it here
        .replace("TWITTER_LINK_HERE", "[twitter](https://x.com/sudolabel)")
}
