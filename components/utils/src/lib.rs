use chrono::{DateTime, Utc};
use ethers::types::U256;
use serde::{de, Deserialize, Deserializer};
use serde_json::Value;

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
