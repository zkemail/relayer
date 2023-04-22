mod halo2_simple;
use async_trait::async_trait;
use ethers::abi::Tokenize;
pub use halo2_simple::*;
use std::path::Path;

use anyhow::Result;

#[async_trait]
pub trait EmailProver {
    type ProofCalldata: Tokenize;
    async fn prove_emails(&mut self) -> Result<()>;
    async fn push_email(&mut self, manipulation_id: usize, email_bytes: &[u8]) -> Result<()>;
    async fn pop_calldata(&mut self) -> Result<Option<(usize, String, Self::ProofCalldata)>>; // (manipulation id, raw email, calldata)
}
