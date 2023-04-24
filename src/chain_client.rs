mod halo2_client;
use crate::prover::EmailProver;
use anyhow::Result;
use async_trait::async_trait;
use ethers::{
    abi::Tokenize,
    types::{H256, U256},
};
pub use halo2_client::*;
use std::path::Path;
#[async_trait]
pub trait ChainClient<P: EmailProver> {
    async fn send_chain(&self, manipulation_id: usize, calldata: P::ProofCalldata) -> Result<H256>; // return transaction hash
    async fn query_balance(&self, email_address: &str, token_name: &str) -> Result<U256>;
}
