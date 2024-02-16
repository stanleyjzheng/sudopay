use anyhow::anyhow;
use bcrypt::{hash, verify};
use ethers::signers::{
    coins_bip39::{English, Mnemonic},
    MnemonicBuilder, Signer,
};
use sqlx::{query, query_as, PgPool};

pub struct User {
    pub telegram_id: i64,
    pub salted_password: Option<String>,
    pub seed_phrase: String,
    pub seed_phrase_public_key: String,
}

impl User {
    pub async fn new(pool: &PgPool, telegram_id: i64) -> anyhow::Result<User> {
        let mnemonic: Mnemonic<English> = Mnemonic::new_with_count(&mut rand::thread_rng(), 12)?;
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(mnemonic.to_phrase().as_str())
            .build()?;
        let address = wallet.address();

        let user = query_as!(
            User,
            "
            INSERT INTO users (
                telegram_id,
                seed_phrase,
                seed_phrase_public_key
            )
            VALUES ($1, $2, $3)
            ON CONFLICT (telegram_id) DO NOTHING
            RETURNING telegram_id, salted_password, seed_phrase, seed_phrase_public_key;
            ",
            telegram_id,
            mnemonic.to_phrase(),
            format!("{:#?}", address)
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user(pool: &PgPool, telegram_id: i64) -> anyhow::Result<Option<User>> {
        let user = query_as!(
            User,
            "
            SELECT telegram_id, salted_password, seed_phrase, seed_phrase_public_key
            FROM users
            WHERE telegram_id = $1;
            ",
            telegram_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    pub async fn check_password(
        &self,
        pool: &PgPool,
        submitted_password: &str,
    ) -> anyhow::Result<bool> {
        let row = query!(
            "SELECT salted_password FROM users WHERE telegram_id = $1",
            self.telegram_id
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row_data) => match row_data.salted_password {
                Some(salted_password) => {
                    let verify = verify(submitted_password, &salted_password)?;
                    Ok(verify)
                }
                // No password set, but user exists
                None => Ok(true),
            },
            // User does not exist
            None => Err(anyhow!("User not found")),
        }
    }

    pub async fn set_password(&mut self, pool: &PgPool, password: &str) -> anyhow::Result<()> {
        let hashed_password = hash(password, bcrypt::DEFAULT_COST)?;
        query!(
            "
            UPDATE users
            SET salted_password = $1
            WHERE telegram_id = $2;
            ",
            hashed_password,
            self.telegram_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
