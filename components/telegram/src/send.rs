use std::str::FromStr as _;

use anyhow::anyhow;
use common::types::SudoPayAsset;
use config::Config;
use ethers::{
    providers::{Http, Provider},
    types::{Address, U256},
};
use sqlx::{types::BigDecimal, PgPool};
use transaction_sender::make_transfer_and_update_balances;

use crate::utils::ens_to_address;

async fn parse_user_response_and_send(
    config: Config,
    provider: &Provider<Http>,
    pool: PgPool,
    input: &str,
    asset: &SudoPayAsset,
    value: BigDecimal,
    seed_phrase_public_key: &str,
) -> anyhow::Result<()> {
    // ens address
    if input.ends_with(".eth") {
        let to_address = ens_to_address(provider.clone(), input.to_string()).await?;

        let u256_value = U256::from_dec_str(&value.to_string())?;

        make_transfer_and_update_balances(
            &config,
            &pool,
            to_address,
            u256_value,
            asset,
            seed_phrase_public_key,
        )
        .await?;
    }
    // telegram username
    else if input.starts_with('@') {
        // handle_telegram_username(input);
    }
    // regular eth address
    else if input.starts_with("0x") && input.len() == 42 {
        let u256_value = U256::from_dec_str(&value.to_string())?;
        let to_address = Address::from_str(input)?;

        make_transfer_and_update_balances(
            &config,
            &pool,
            to_address,
            u256_value,
            asset,
            seed_phrase_public_key,
        )
        .await?;
    } else {
        return Err(anyhow!("Invalid input format."));
    }

    Ok(())
}
