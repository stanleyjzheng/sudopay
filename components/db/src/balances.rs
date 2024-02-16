use std::str::FromStr;

use anyhow::anyhow;
use chrono::NaiveDateTime;
use common::types::SudoPayAsset;
use once_cell::sync::Lazy;
use sqlx::{query, query_as, types::BigDecimal, PgPool};

pub static ANNUAL_YIELD_RATE: Lazy<BigDecimal> =
    Lazy::new(|| BigDecimal::from_str("0.035").unwrap());

pub struct Balance {
    pub seed_phrase_public_key: String,
    pub usdb_balance: BigDecimal,
    pub eth_balance: BigDecimal,
    pub accrued_yield_balance: BigDecimal,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Balance {
    pub async fn new(pool: &PgPool, seed_phrase_public_key: String) -> anyhow::Result<Self> {
        let balance = query_as!(
            Balance,
            "INSERT INTO balances (seed_phrase_public_key)
             VALUES ($1)
             ON CONFLICT (seed_phrase_public_key) DO NOTHING
             RETURNING seed_phrase_public_key, usdb_balance, eth_balance, accrued_yield_balance, created_at, updated_at",
            seed_phrase_public_key,
        )
        .fetch_one(pool)
        .await?;

        Ok(balance)
    }

    async fn update_accrued_yield(
        pool: &sqlx::Pool<sqlx::Postgres>,
        seed_phrase_public_key: &str,
    ) -> anyhow::Result<()> {
        // For every balance update, we calculate the yield accrual based on each balance
        // since the last updated time. For now, these are consts set slightly below actual yield.

        sqlx::query!(
            "UPDATE balances
             SET accrued_yield_balance = accrued_yield_balance + CAST(((eth_balance * $2 / 365.0 / 24.0 / 3600.0) * (EXTRACT(EPOCH FROM NOW()) - EXTRACT(EPOCH FROM updated_at))) AS BIGINT),
                 updated_at = NOW()
             WHERE seed_phrase_public_key = $1",
            seed_phrase_public_key,
            &ANNUAL_YIELD_RATE as &BigDecimal
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn add_to_balance(
        pool: &PgPool,
        seed_phrase_public_key: &str,
        amount: BigDecimal,
        asset: &SudoPayAsset,
    ) -> anyhow::Result<()> {
        match asset.to_owned() {
            SudoPayAsset::Eth | SudoPayAsset::Weth => {
                query!(
                    "UPDATE balances SET eth_balance = eth_balance + $1 WHERE seed_phrase_public_key = $2",
                    amount,
                    seed_phrase_public_key
                )
                .execute(pool)
                .await?;
            }
            SudoPayAsset::Usdb => {
                query!(
                    "UPDATE balances SET usdb_balance = usdb_balance + $1 WHERE seed_phrase_public_key = $2",
                    amount,
                    seed_phrase_public_key
                )
                .execute(pool)
                .await?;
            }
        }

        // Update accrued yield after balance change
        Self::update_accrued_yield(pool, seed_phrase_public_key).await?;

        Ok(())
    }

    pub async fn subtract_from_balance(
        pool: &PgPool,
        seed_phrase_public_key: &str,
        amount: BigDecimal,
        asset: &SudoPayAsset,
    ) -> anyhow::Result<()> {
        match asset.to_owned() {
            SudoPayAsset::Eth | SudoPayAsset::Weth => {
                let result = query!(
                    "UPDATE balances SET eth_balance = eth_balance - $1 WHERE seed_phrase_public_key = $2",
                    amount,
                    seed_phrase_public_key
                )
                .execute(pool)
                .await?;

                if result.rows_affected() == 0 {
                    return Err(anyhow!("Balance row not found"));
                }
            }
            SudoPayAsset::Usdb => {
                let result = query!(
                    "UPDATE balances SET usdb_balance = usdb_balance - $1 WHERE seed_phrase_public_key = $2",
                    amount,
                    seed_phrase_public_key
                )
                .execute(pool)
                .await?;

                if result.rows_affected() == 0 {
                    return Err(anyhow!("Balance row not found"));
                }
            }
        }

        // Update accrued yield after balance change
        Self::update_accrued_yield(pool, seed_phrase_public_key).await?;

        Ok(())
    }

    pub async fn get_by_seed_phrase_public_key(
        pool: &PgPool,
        seed_phrase_public_key: &str,
    ) -> anyhow::Result<Self> {
        let balance = query_as!(
            Balance,
            "SELECT seed_phrase_public_key, usdb_balance, eth_balance, accrued_yield_balance, created_at, updated_at
             FROM balances
             WHERE seed_phrase_public_key = $1",
            seed_phrase_public_key,
        )
        .fetch_one(pool)
        .await?;

        Ok(balance)
    }
}
