// create a new user without password if the password is empty
// get the hash if it exists
// check it against user's password
use bcrypt::{hash, verify};
use ethers::signers::{
    coins_bip39::{English, Mnemonic},
    MnemonicBuilder, Signer,
};
use sqlx::{query, query_as, PgConnection};

pub struct User {
    pub telegram_id: i64,
    pub salted_password: Option<String>,
    pub seed_phrase: String,
    pub seed_phrase_public_key: String,
}

impl User {
    pub async fn new(conn: &mut PgConnection, telegram_id: i64) -> anyhow::Result<User> {
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
        .fetch_one(&mut *conn)
        .await?;

        Ok(user)
    }

    pub async fn get_user(conn: &mut PgConnection, telegram_id: i64) -> anyhow::Result<User> {
        let user = query_as!(
            User,
            "
            SELECT telegram_id, salted_password, seed_phrase, seed_phrase_public_key
            FROM users
            WHERE telegram_id = $1;
            ",
            telegram_id
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok(user)
    }

    pub async fn check_password(
        &self,
        conn: &mut PgConnection,
        submitted_password: &str,
    ) -> anyhow::Result<bool> {
        let row = query!(
            "SELECT salted_password FROM users WHERE telegram_id = $1",
            self.telegram_id
        )
        .fetch_one(&mut *conn)
        .await?;

        let verify = verify(submitted_password, &row.salted_password.unwrap_or_default())?;

        Ok(verify)
    }

    pub async fn set_password(
        &mut self,
        conn: &mut PgConnection,
        password: &str,
    ) -> anyhow::Result<()> {
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
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}
