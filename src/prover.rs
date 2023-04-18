use std::path::Path;

use anyhow::Result;
pub trait EmailProver {
    type ProofSet;
    fn prove_emails(&mut self) -> Result<()>;
    fn emails_dir_path(&self) -> Path;
    fn num_proofs_per_aggregation(&self) -> usize;
    fn push_email(&mut self, email_bytes: &[u8]) -> Result<()>;
    fn pop_proof(&mut self) -> Result<Self::ProofSet>;
}
