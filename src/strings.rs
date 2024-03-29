use ethers::core::types::{Address, U256, H160, H256};
use crate::chain::{query_balance};
pub const CHAIN: &str = "Ethereum Goerli";

pub fn invalid_reply() -> String {
    format!(
        "Subject failed formatting check! Please format your email on https://sendeth.org, or try again with this subject: \"Send _ DAI to __@__.___\". \
        You can send DAI, USDC, or TEST tokens right now."
    )
}

pub fn bad_message_id() -> String {
    format!(
        "Email did not have a message-id! Your email client may not be supported -- please contact us at aayushg@mit.edu for us to add support for your domain."
    )
}

pub fn reply_with_etherscan(tx_hash: H256) -> String {
    let etherscan_url = format!("https://goerli.etherscan.io/tx/0x{:x}", tx_hash);
    let reply = format!(
        "Transaction sent! View Etherscan confirmation: {}.\n \n\
        If either email address is new, we've assigned them an address on-chain, controlled only by that email (your email address is not leaked on-chain).\n \n\
        Under 'ERC20 Tokens Transferred', you'll see transactions in which we give each new wallet 100 tokens, and the last line shows your tokens moving between the two accounts.",
        etherscan_url
    );
    println!("Replying with confirmation...{}", reply);
    reply
}

pub async fn pending_reply(address: &str, amount: &str, currency: &str, recipient: &str) -> String {
    let mut enough_balance = false;
    let balance_detected_message = match query_balance(
        false,
        address.clone(),
        currency,
    )
    .await {
        Ok(balance) => {
            enough_balance = balance >= amount.parse().unwrap();
            let remaining = balance - amount.parse::<f64>().unwrap();
            
            let enough_balance_str = 
                if (currency == "TEST" && remaining == 100.0) {
                    format!("Created new wallet for you at {}, controlled by your emails. Your email address is not leaked on-chain. It has {} {}, and the transaction will send {} {} to {} and your remaining balance will be {} {}.", address, balance, currency, amount, currency, recipient, remaining, currency)                
                } else if enough_balance {
                    format!("Your wallet {} has {} {}. The transaction will send {} {} to {} and your remaining balance will be {} {}.", address, balance, currency, amount, currency, recipient, remaining, currency)
                } else {
                    format!("Created new wallet for you at {} -- in order to send this transaction, you must add at least {} {} to send. \
                    The send has been queued and will execute once enough balance is detected, then automatically send {} {} to {}.",
                    address, amount, currency, amount, currency, recipient)
                };
            enough_balance_str
        },
        Err(_) => {
            format!("Failed to detect balance in account.")
        }
    };
    println!("Balance detected message: {}", balance_detected_message);
       

    format!(
        "{} \
        We will follow up with {} Etherscan link in about a minute when finished. \n \n\
        You are sending using zk email (https://prove.email) and email wallet (https://emailwallet.org). \
        The relayer will prove on-chain that you sent an email authorizing this transaction. \
        We will automatically deploy a wallets for each new user, controlled only by that new user's email address and domain (we can't steal your assets!). \
        While we're in beta, we transfer you 100 'TEST' tokens to try out free transfers.",
        balance_detected_message, CHAIN
    )
}

pub fn recipient_intro_body(sender_email: &str, amount: &str, currency: &str) -> String {
    format!(
        "You have received a transfer from {} for {} {} on {}. \
        We automatically created a wallet for you and sent you the money using Email Wallet's ZK technology (https://prove.email). 
        
        If you want to transfer these funds or cash out, you just need to send another email, which you can format on https://emailwallet.org.
        
        If you don't want this money or weren't expecting a transfer, you can ignore this email, and the money will automatically be returned once a month has passed.",
        sender_email, amount, currency, CHAIN
    )
}

// TODO: Change view > claim for uninitialized accounts
pub fn recipient_intro_subject(sender_email: &str, amount: &str, currency: &str) -> String {
    format!(
        "View your transfer from {} for {} {} on {}",
        sender_email, amount, currency, CHAIN
    )
}