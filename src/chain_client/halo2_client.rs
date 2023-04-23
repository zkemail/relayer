use std::str::FromStr;

use crate::chain_client::ChainClient;
use crate::prover::{EmailProver, Halo2SimpleProver};
use anyhow::Result;
use async_trait::async_trait;
use ethers::abi::Abi;
use ethers::prelude::*;
use ethers::providers::Provider;
use std::fs;

#[derive(Debug, Clone)]
pub struct Halo2Client {
    contract_address: H160,
    abi: Abi,
    signer: SignerMiddleware<Provider<Http>, LocalWallet>,
}

#[async_trait]
impl ChainClient<Halo2SimpleProver> for Halo2Client {
    async fn send_chain(
        &self,
        manipulation_id: usize,
        calldata: <Halo2SimpleProver as EmailProver>::ProofCalldata,
    ) -> Result<H256> {
        let gas_price = self.signer.provider().get_gas_price().await?;
        let (max_fee_per_gas, _) = self.signer.provider().estimate_eip1559_fees(None).await?;
        // let provider = self.provider.clone();
        // let signer = SignerMiddleware::new(self.provider, self.wallet.with_chain_id(self.chain_id));
        let contract = ContractInstance::<_, SignerMiddleware<Provider<Http>, LocalWallet>>::new(
            self.contract_address,
            self.abi.clone(),
            &self.signer,
        );

        println!("Sending transaction with gas price {:?}...", gas_price);

        // Call the process function
        let (param, acc, proof) = calldata;
        let call = contract
            .method::<_, ()>("process", (U256::from(manipulation_id), param, acc, proof))?
            .gas_price(max_fee_per_gas); // Set an appropriate gas limit

        println!("Calling call: {:?}", call);

        let pending_tx = match call.send().await {
            Ok(tx) => tx,
            Err(e) => {
                println!("Error: {:?}", e);
                return Err(e.into());
            }
        };
        println!("Transaction hash: {:?}", pending_tx);
        Ok(pending_tx.tx_hash())
    }
}

impl Halo2Client {
    pub fn construct(
        private_key_hex: &str,
        rpc_url: &str,
        contract_address: &str,
        abi_path: &str,
        chain_id: u64,
    ) -> Result<Self> {
        let wallet = LocalWallet::from_str(&private_key_hex)?;
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let signer = SignerMiddleware::new(provider, wallet.with_chain_id(chain_id));
        let contract_address = H160::from_str(contract_address)?;
        let abi_str = fs::read_to_string(abi_path)?;
        // Parse the string as JSON to obtain the ABI
        let abi: Abi = serde_json::from_str(abi_str.as_str())?;
        Ok(Self {
            contract_address,
            abi,
            signer,
        })
    }
}
