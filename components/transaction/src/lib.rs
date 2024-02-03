use dotenv::dotenv;
use ethers::prelude::{Http, LocalWallet, Provider, TransactionRequest};
use std::env;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not found in .env");
    let rpc_url = "your_custom_rpc_url";
    let recipient_address = "recipient_eth_address";
    let amount_in_wei = 1000000000000000000u64; // 1 ETH

    let wallet = LocalWallet::from_str(&private_key)?;
    let provider = Provider::<Http>::try_from(rpc_url)?.with_sender(wallet);

    let tx = TransactionRequest::new()
        .to(recipient_address)
        .value(amount_in_wei);

    let tx_hash = provider.send_transaction(tx, None).await??;
    println!("Transaction hash: {:?}", tx_hash);

    Ok(())
}
