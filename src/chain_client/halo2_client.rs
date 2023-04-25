use std::str::FromStr;

use crate::chain_client::ChainClient;
use crate::prover::{EmailProver, Halo2SimpleProver};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethers::abi::{Abi, Address};
use ethers::prelude::*;
use ethers::providers::Provider;
use std::fs;

#[derive(Debug, Clone)]
pub struct Halo2Client {
    contract_address: H160,
    wallet_abi: Abi,
    erc20_abi: Abi,
    iman_abi: Abi,
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
        let wallet_contract =
            ContractInstance::<_, SignerMiddleware<Provider<Http>, LocalWallet>>::new(
                self.contract_address,
                self.wallet_abi.clone(),
                &self.signer,
            );
        let manipulator_address = wallet_contract
            .method::<_, Address>("manipulationOfId", U256::from(manipulation_id))?
            .call()
            .await?;
        let manipulator = ContractInstance::<_, SignerMiddleware<Provider<Http>, LocalWallet>>::new(
            manipulator_address,
            self.iman_abi.clone(),
            &self.signer,
        );
        let is_valid = manipulator
            .method::<_, bool>("verifyWrap", calldata.clone())?
            .call()
            .await?;
        if !is_valid {
            return Err(anyhow!("invalid proof"));
        }

        println!("Sending transaction with gas price {:?}...", gas_price);

        // Call the process function
        let (param, acc, proof) = calldata;
        let call = wallet_contract
            .method::<_, ()>("process", (U256::from(manipulation_id), param, acc, proof))?
            .legacy()
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

    async fn query_balance(&self, email_address: &str, token_name: &str) -> Result<String> {
        // let provider = self.provider.clone();
        // let signer = SignerMiddleware::new(self.provider, self.wallet.with_chain_id(self.chain_id));
        let wallet_contract =
            ContractInstance::<_, SignerMiddleware<Provider<Http>, LocalWallet>>::new(
                self.contract_address,
                self.wallet_abi.clone(),
                &self.signer,
            );
        let token_address = wallet_contract
            .method::<_, Address>("erc20OfTokenName", token_name.to_string())?
            .call()
            .await?;
        let erc20_contract =
            ContractInstance::<_, SignerMiddleware<Provider<Http>, LocalWallet>>::new(
                token_address,
                self.erc20_abi.clone(),
                &self.signer,
            );
        let decimals: u8 = erc20_contract
            .method::<_, Uint8>("decimals", ())?
            .call()
            .await?
            .into();
        let balance_int = wallet_contract
            .method::<_, U256>(
                "balanceOfUser",
                (email_address.to_string(), token_name.to_string()),
            )?
            .call()
            .await?;
        let balance_f64 = f64::from_str(&balance_int.to_string())?;
        let actual_f64 = balance_f64 / (10f64.powi(decimals as i32));
        Ok(actual_f64.to_string())
    }
}

impl Halo2Client {
    pub fn construct(
        private_key_hex: &str,
        rpc_url: &str,
        contract_address: &str,
        wallet_abi_path: &str,
        erc20_abi_path: &str,
        iman_abi_path: &str,
        chain_id: u64,
    ) -> Result<Self> {
        let wallet = LocalWallet::from_str(&private_key_hex)?;
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let signer = SignerMiddleware::new(provider, wallet.with_chain_id(chain_id));
        let contract_address = H160::from_str(contract_address)?;
        let wallet_abi_str = fs::read_to_string(wallet_abi_path)?;
        let wallet_abi: Abi = serde_json::from_str(wallet_abi_str.as_str())?;
        let erc20_abi_str = fs::read_to_string(erc20_abi_path)?;
        let erc20_abi: Abi = serde_json::from_str(erc20_abi_str.as_str())?;
        let iman_abi_str = fs::read_to_string(iman_abi_path)?;
        let iman_abi: Abi = serde_json::from_str(iman_abi_str.as_str())?;
        Ok(Self {
            contract_address,
            wallet_abi,
            erc20_abi,
            iman_abi,
            signer,
        })
    }
}
