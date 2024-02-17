use common::types::SudoPayAsset;
use ethers::types::H160;
use sqlx::types::BigDecimal;
use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    utils::command::BotCommands,
};

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    // Deposits
    AwaitingDepositAddress,
    AwaitingDepositAmount,
    UserClickedDeposit,
    UserInputtedDepositAddress {
        user_address: H160,
    },
    UserInputtedDepositAmount {
        deposit_amount: BigDecimal,
    },
    // Send
    AwaitingSendAddress,
    UserInputtedSendAddress {
        address_or_handle: String,
    },
    UserInputtedAssetAndAddress {
        asset: SudoPayAsset,
        address_or_handle: String,
    },
    AwaitingSendAmount {
        asset: SudoPayAsset,
        address_or_handle: String,
    },
    UserInputtedAssetAddressAndAmount {
        asset: SudoPayAsset,
        amount: f64,
        address_or_handle: String,
    },
}

/// These commands are supported:
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Main menu
    Start,
    /// Display this text.
    Help,
    /// Start a payment, whether now or later.
    Pay,
    /// Make a deposit
    Deposit,
    /// Make a withdrawal
    Withdraw,
    /// Cancel the transaction procedure.
    Cancel,
    /// User settings
    Settings,
    /// Pay now
    Now,
    /// Schedule a payment
    Later,
    /// Go back
    Back,
    /// Go to main menu
    Menu,
    /// List all transactions
    ListTransactions,
    /// List all deposits
    ListDeposits,
    /// List all withdrawals
    ListWithdrawals,
    /// Send
    Send,
}

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
