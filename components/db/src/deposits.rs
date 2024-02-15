use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use common::types::SudoPayAsset;
use once_cell::sync::Lazy;
use sqlx::{query, query_scalar, types::BigDecimal, FromRow, PgPool, Postgres, Row, Transaction};

static LOWER_BOUND_EPSILON: Lazy<BigDecimal> = Lazy::new(|| BigDecimal::from_str("0.999").unwrap());
static UPPER_BOUND_EPSILON: Lazy<BigDecimal> =
    Lazy::new(|| BigDecimal::from_str("0.1001").unwrap());

#[derive(FromRow)]
pub struct Deposit {
    pub transaction_id: String,
    pub transaction_from_public_key: String,
    pub asset: SudoPayAsset,
    pub amount: BigDecimal,
    pub matched: bool,
    pub created_at: DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct NewDeposit {
    pub transaction_id: String,
    pub transaction_from_public_key: String,
    pub asset: SudoPayAsset,
    pub amount: BigDecimal,
    pub created_at: DateTime<chrono::Utc>,
}

impl Deposit {
    // Constructor for creating a new Deposit
    pub async fn new(
        pool: &PgPool,
        transaction_id: String,
        transaction_from_public_key: String,
        asset: SudoPayAsset,
        amount: BigDecimal,
    ) -> anyhow::Result<Self> {
        query!(
            "INSERT INTO deposits (transaction_id, transaction_from_public_key, asset, amount) VALUES ($1, $2, $3, $4) ON CONFLICT (transaction_id) DO NOTHING",
            transaction_id,
            transaction_from_public_key,
            asset.to_string(),
            amount
        )
        .execute(pool)
        .await?;

        Ok(Self {
            transaction_id,
            transaction_from_public_key,
            asset,
            amount,
            matched: false,
            created_at: Utc::now(),
        })
    }

    pub async fn any_transaction_id_exists(
        pool: &PgPool,
        transaction_ids: &[String],
    ) -> anyhow::Result<bool> {
        // Prepare a query string with ANY($1) where $1 will be replaced by the list of transaction IDs
        let query = query_scalar!(
            "SELECT EXISTS (
                SELECT 1 FROM deposits 
                WHERE transaction_id = ANY($1)
            )",
            transaction_ids
        );

        let exists: bool = query.fetch_one(pool).await?.is_some();

        Ok(exists)
    }

    pub async fn all_transaction_ids_exist(
        pool: &PgPool,
        transaction_ids: &[String],
    ) -> anyhow::Result<bool> {
        // Calculate the number of unique transaction_ids in the input list
        let unique_ids_count = transaction_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();

        let query = query_scalar!(
            "SELECT COUNT(DISTINCT transaction_id) FROM deposits WHERE transaction_id = ANY($1)",
            transaction_ids
        );

        let count: i64 = query.fetch_one(pool).await?.unwrap_or_default();

        Ok(count as usize == unique_ids_count)
    }

    pub async fn set_deposit_matched(pool: &PgPool, transaction_id: &str) -> anyhow::Result<()> {
        query!(
            "UPDATE deposits SET matched = TRUE WHERE transaction_id = $1",
            transaction_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn insert_bulk_deposits(
        pool: &mut PgPool,
        deposits: Vec<NewDeposit>,
    ) -> anyhow::Result<Vec<Deposit>> {
        let mut tx: Transaction<Postgres> = pool.begin().await?;
        let mut inserted_deposits = Vec::new();

        for deposit in deposits {
            query!(
                "INSERT INTO deposits (transaction_id, transaction_from_public_key, asset, amount, created_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (transaction_id) DO NOTHING",
                deposit.transaction_id,
                deposit.transaction_from_public_key,
                deposit.asset.to_string(),
                deposit.amount,
                deposit.created_at.naive_utc()
            )
            .execute(&mut *tx)
            .await?;

            inserted_deposits.push(Deposit {
                transaction_id: deposit.transaction_id,
                transaction_from_public_key: deposit.transaction_from_public_key,
                asset: deposit.asset,
                amount: deposit.amount,
                matched: false,
                created_at: deposit.created_at,
            });
        }

        tx.commit().await?;

        Ok(inserted_deposits)
    }

    pub async fn filter_non_existing_transaction_ids(
        pool: &PgPool,
        transaction_ids: &[String],
    ) -> anyhow::Result<Vec<String>> {
        let query_string = r#"
            WITH input_ids(id) AS (
                SELECT unnest($1::text[]) 
            )
            SELECT input_ids.id
            FROM input_ids
            LEFT JOIN transactions ON transactions.id = input_ids.id
            WHERE transactions.id IS NULL
        "#;

        let result = query(query_string)
            .bind(transaction_ids)
            .fetch_all(pool)
            .await?;

        let non_existing_ids: Vec<String> = result.into_iter().map(|row| row.get(0)).collect();

        Ok(non_existing_ids)
    }
}

#[derive(FromRow, Clone)]
pub struct DepositRequest {
    pub id: i32,
    pub depositor_public_key: String,
    pub asset: SudoPayAsset,
    pub unit_amount: Option<BigDecimal>,
    pub from_address: Option<String>,
    pub matched_transaction_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl DepositRequest {
    // Constructor for creating a new DepositRequest
    pub async fn new(
        pool: &PgPool,
        depositor_public_key: String,
        asset: SudoPayAsset,
        unit_amount: Option<BigDecimal>,
        from_address: Option<String>,
    ) -> anyhow::Result<Self> {
        let rec = query!(
            "INSERT INTO deposit_requests (depositor_public_key, asset, unit_amount, from_address) VALUES ($1, $2, $3, $4) RETURNING id, created_at, updated_at",
            depositor_public_key,
            asset.to_string(),
            unit_amount,
            from_address,
        )
        .fetch_one(pool)
        .await?;

        Ok(Self {
            id: rec.id,
            depositor_public_key,
            asset,
            unit_amount,
            from_address,
            matched_transaction_id: None,
            created_at: rec.created_at,
            updated_at: rec.updated_at,
        })
    }

    // Fetch a DepositRequest by its ID
    pub async fn get_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Self> {
        let rec = query!("SELECT * FROM deposit_requests WHERE id = $1", id)
            .fetch_one(pool)
            .await?;

        let asset: SudoPayAsset = rec.asset.parse()?;

        let deposit_request = DepositRequest {
            id: rec.id,
            depositor_public_key: rec.depositor_public_key,
            asset,
            unit_amount: rec.unit_amount,
            from_address: rec.from_address,
            matched_transaction_id: rec.matched_transaction_id,
            created_at: rec.created_at,
            updated_at: rec.updated_at,
        };

        Ok(deposit_request)
    }

    // Fetch DepositRequests by amount within a specified time range, allowing a 1% range on either side
    pub async fn from_amount_by_time(
        pool: &PgPool,
        amount: BigDecimal,
        asset: SudoPayAsset,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> anyhow::Result<Vec<Self>> {
        let lower_bound = &amount * &*LOWER_BOUND_EPSILON;
        let upper_bound = &amount * &*UPPER_BOUND_EPSILON;

        let recs = query!(
            "SELECT * FROM deposit_requests WHERE unit_amount BETWEEN $1 AND $2 AND created_at BETWEEN $3 AND $4 AND asset = $5",
            lower_bound,
            upper_bound,
            start_time.naive_utc(),
            end_time.naive_utc(),
            asset.to_string()
        )
        .fetch_all(pool)
        .await?;

        let deposit_requests: Vec<DepositRequest> = recs
            .iter()
            .filter_map(|rec| {
                let asset = rec.asset.parse();
                match asset {
                    Ok(asset) => Some(DepositRequest {
                        id: rec.id,
                        depositor_public_key: rec.depositor_public_key.clone(),
                        asset,
                        unit_amount: rec.unit_amount.clone(),
                        from_address: rec.from_address.clone(),
                        matched_transaction_id: rec.matched_transaction_id.clone(),
                        created_at: rec.created_at,
                        updated_at: rec.updated_at,
                    }),
                    Err(_) => {
                        log::error!("Failed to parse asset from string: {}", rec.asset);
                        None
                    }
                }
            })
            .collect();

        Ok(deposit_requests)
    }

    // Fetch DepositRequests by from_address within a specified time range
    pub async fn from_address_by_time(
        pool: &PgPool,
        from_address: String,
        asset: SudoPayAsset,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> anyhow::Result<Vec<Self>> {
        let recs = query!(
            "SELECT * FROM deposit_requests WHERE from_address = $1 AND created_at BETWEEN $2 AND $3 AND asset = $4",
            from_address,
            start_time.naive_utc(),
            end_time.naive_utc(),
            asset.to_string()
        )
        .fetch_all(pool)
        .await?;

        let deposit_requests: Vec<DepositRequest> = recs
            .iter()
            .filter_map(|rec| {
                let asset = rec.asset.parse();
                match asset {
                    Ok(asset) => Some(DepositRequest {
                        id: rec.id,
                        depositor_public_key: rec.depositor_public_key.clone(),
                        asset,
                        unit_amount: rec.unit_amount.clone(),
                        from_address: rec.from_address.clone(),
                        matched_transaction_id: rec.matched_transaction_id.clone(),
                        created_at: rec.created_at,
                        updated_at: rec.updated_at,
                    }),
                    Err(_) => {
                        log::error!("Failed to parse asset from string: {}", rec.asset);
                        None
                    }
                }
            })
            .collect();

        Ok(deposit_requests)
    }

    // Update the matched_transaction_id for a DepositRequest
    pub async fn set_matched_transaction_id(
        pool: &PgPool,
        id: i32,
        matched_transaction_id: String,
    ) -> Result<(), sqlx::Error> {
        query!(
            "UPDATE deposit_requests SET matched_transaction_id = $1 WHERE id = $2",
            matched_transaction_id,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
