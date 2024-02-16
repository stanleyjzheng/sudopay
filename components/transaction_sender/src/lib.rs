use config::Config;
use ethers::abi::parse_abi;
use ethers::prelude::*;
use ethers::signers::LocalWallet;
use std::convert::TryFrom;
use std::sync::Arc;

// TODO: make this and erc20 transfers less repetitive, but it moves a Box<Error> across threads so it's a bit of a pain
// to move the `client` or `contract` logic outside of this function
pub async fn make_eth_transfer(
    config: &Config,
    to_address: Address,
    value: U256,
) -> anyhow::Result<TxHash> {
    let provider = Provider::<Http>::try_from(config.http_rpc_url.clone())?;

    let wallet = config
        .transferrer_private_key
        .parse::<LocalWallet>()?
        .with_chain_id(config.chain_id);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    let abi = parse_abi(&[
        "function transferETH(address payable _to, uint256 _amount) external",
        "function transferERC20(address _token, address _to, uint256 _amount) external        ",
    ])
    .unwrap();
    let contract_address: Address = config.contract_address.parse()?;
    let contract = Contract::new(contract_address, abi, client);

    let amount_eth = value;

    // Send the transaction and immediately get the hash
    let binding = contract.method::<_, H256>("transferETH", (to_address, amount_eth))?;
    let tx_request = binding.send().await?;

    Ok(tx_request.tx_hash())
}

pub async fn make_erc20_transfer(
    config: &Config,
    erc20_contract_address: Address,
    to_address: Address,
    value: U256,
) -> anyhow::Result<TxHash> {
    let provider = Provider::<Http>::try_from(config.http_rpc_url.clone())?;

    let wallet = config
        .transferrer_private_key
        .parse::<LocalWallet>()?
        .with_chain_id(config.chain_id);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    let abi = parse_abi(&[
        "function transferETH(address payable _to, uint256 _amount) external",
        "function transferERC20(address _token, address _to, uint256 _amount) external        ",
    ])
    .unwrap();
    let contract_address: Address = config.contract_address.parse()?;
    let contract = Contract::new(contract_address, abi, client);

    let amount = value;

    // Send the transaction and immediately get the hash without confirmations
    let binding = contract.method::<_, H256>(
        "transferERC20",
        (erc20_contract_address, to_address, amount),
    )?;
    let tx_request = binding.send().await?;

    Ok(tx_request.tx_hash())
}

#[tokio::test]
async fn it_makes_testnet_eth_transfers() -> anyhow::Result<()> {
    let config = Config::new_from_env();

    make_eth_transfer(
        &config,
        "0xBeafFE58538eAfe49d1E4455500BC659f5D37433".parse()?,
        U256::from(100000000000000000_u64),
    )
    .await?;

    Ok(())
}
