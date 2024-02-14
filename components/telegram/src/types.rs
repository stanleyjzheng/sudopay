use ethers::types::H160;
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
        deposit_amount: f64,
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
}

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
