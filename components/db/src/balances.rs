pub struct DepositRequest {
    pub id: i32,
    pub amount: Option<i32>,
    pub from_address: Option<String>,
    pub currency: String,
    pub tx_hash: String,
    pub created_at: chrono::NaiveDateTime,
    pub matched: bool,
}

pub enum DepositType {
    Amount,
    Contract,
}