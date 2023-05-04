pub fn first_reply(amount: &str, recipient: &str) -> String {
    format!(
        "Valid send initiated. Sending {} TestERC20 to {} on Ethereum. \
        We will follow up with Etherscan link when finished! You are sending with ZK email technology. \
        The relayer will automatically prove, on-chain, that you sent an email authorizing this transaction.  \
        We will deploy a wallet for you if you don't have one. While we're in beta, we'll also give you \
        10 TestERC20 if you don't have any to start, but in the future when we use real currency, \
        we'll send you an address to top up your wallet via Ethereum or other methods.",
        amount, recipient
    )
}

pub fn invalid_reply(reason: &str) -> String {
    format!(
        "Send {}! Please try again with this subject: \"Send _ dai to __@__.___\". \
    You can send dai or eth right now, but it all sends with TestERC20.",
        reason
    )
}
