use crate::config::ManipulationDefsJson;
use crate::config::RegexType;
use crate::prover::EmailProver;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethers::abi::FixedBytes;
use ethers::abi::Token;
use ethers::abi::{ParamType, SolStruct};
use ethers::prelude::*;
use ethers::{abi::Tokenize, types::Bytes};
use fancy_regex::Regex;
use halo2_zk_email::{evm_prove_agg, EmailVerifyPublicInput};
use std::fs::{self, File};
use std::io::Write;
use std::str::FromStr;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Halo2SimpleProver {
    email_dir: String,
    app_params_path: String,
    agg_params_path: String,
    manipulation_defs: ManipulationDefsJson,
    num_email: usize,
    next_prove_nonce: usize,
    next_pop_nonce: usize,
    id_of_nonce: HashMap<usize, usize>,
}

#[async_trait]
impl EmailProver for Halo2SimpleProver {
    type ProofCalldata = (Bytes, Bytes, Bytes);
    async fn prove_emails(&mut self) -> Result<()> {
        while self.next_prove_nonce < self.num_email {
            self.prove_email(self.next_prove_nonce).await?;
            self.next_prove_nonce += 1;
        }
        Ok(())
    }

    async fn push_email(&mut self, manipulation_id: usize, email_bytes: &[u8]) -> Result<()> {
        let nonce = self.num_email;
        let (email_path, _, _, _) = self.nonce2pathes(nonce);
        let mut file = File::create(email_path)?;
        write!(file, "{}", String::from_utf8(email_bytes.to_vec())?)?;
        file.flush()?;
        self.id_of_nonce.insert(nonce, manipulation_id);
        self.num_email += 1;
        Ok(())
    }

    async fn pop_calldata(&mut self) -> Result<Option<(usize, String, Self::ProofCalldata)>> {
        if self.next_pop_nonce >= self.num_email {
            return Ok(None);
        }
        let nonce = self.next_pop_nonce;
        let (email_path, acc_path, proof_path, public_input_path) = self.nonce2pathes(nonce);
        let email = fs::read_to_string(email_path)?;
        let acc = Bytes::from_str(fs::read_to_string(&acc_path)?.as_str())?;
        let proof = Bytes::from_str(fs::read_to_string(&proof_path)?.as_str())?;
        let public_input = serde_json::from_reader::<File, EmailVerifyPublicInput>(
            File::open(public_input_path).unwrap(),
        )
        .unwrap();
        let id = self.id_of_nonce[&nonce];
        let mut tokens = vec![
            Token::FixedBytes(FixedBytes::from(hex::decode(
                &public_input.headerhash[2..],
            )?)),
            Token::Bytes(hex::decode(&public_input.public_key_n_bytes[2..])?),
            Token::Uint(U256::from(public_input.header_starts[0])),
            Token::String(public_input.header_substrs[0].to_string()),
            Token::Uint(U256::from(public_input.header_starts[1])),
            Token::String(public_input.header_substrs[1].to_string()),
            Token::Uint(U256::from(public_input.header_starts[2])),
            Token::String(public_input.header_substrs[2].to_string()),
            Token::Uint(U256::from(public_input.header_starts[3])),
            Token::Uint(U256::from(id)),
        ];
        let def = &self.manipulation_defs.rules[&id];
        for (idx, regex_type) in def.types.iter().enumerate() {
            tokens.push(Token::Uint(U256::from(public_input.body_starts[idx])));
            match *regex_type {
                RegexType::String => {
                    tokens.push(Token::String(public_input.body_substrs[idx].to_string()));
                }
                RegexType::Uint => tokens.push(Token::Uint(U256::from_str_radix(
                    public_input.body_substrs[idx].as_str(),
                    10,
                )?)),
                RegexType::Decimal => {
                    let int_part = Regex::new(r"[0-9]+(?=\.)")?;
                    let dec_part = Regex::new(r"(?<=\.)[0-9]+")?;
                    let int_found = int_part
                        .find(public_input.body_substrs[idx].as_str())?
                        .ok_or(anyhow!("the int part is not found in {}-th body.", idx))?;
                    let dec_found = dec_part
                        .find(public_input.body_substrs[idx].as_str())?
                        .ok_or(anyhow!("the int part is not found in {}-th body.", idx))?;
                    tokens.push(Token::Uint(U256::from_str_radix(int_found.as_str(), 10)?));
                    tokens.push(Token::Uint(U256::from_str_radix(dec_found.as_str(), 10)?));
                }
            }
        }
        let params_byte = Bytes::from(abi::encode(&[Token::Tuple(tokens)]));
        let calldata = (params_byte, acc, proof);
        self.next_pop_nonce += 1;
        Ok(Some((id, email, calldata)))
    }
}

impl Halo2SimpleProver {
    pub fn construct(
        email_dir: &str,
        app_params_path: &str,
        agg_params_path: &str,
        manipulation_defs_path: &str,
    ) -> Result<Self> {
        let manipulation_defs = serde_json::from_reader::<File, ManipulationDefsJson>(File::open(
            manipulation_defs_path,
        )?)?;
        Ok(Self {
            email_dir: email_dir.to_string(),
            app_params_path: app_params_path.to_string(),
            agg_params_path: agg_params_path.to_string(),
            manipulation_defs,
            num_email: 0,
            next_prove_nonce: 0,
            next_pop_nonce: 0,
            id_of_nonce: HashMap::new(),
        })
    }

    async fn prove_email(&self, nonce: usize) -> Result<()> {
        let (email_path, acc_path, proof_path, public_input_path) = self.nonce2pathes(nonce);
        let id = self.id_of_nonce[&nonce];
        let def = &self.manipulation_defs.rules[&id];
        let app_circuit_config_path = def.app_config_path.as_str();
        let agg_circuit_config_path = def.agg_config_path.as_str();
        let app_pk_path = def.app_pk_path.as_str();
        let agg_pk_path = def.agg_pk_path.as_str();
        evm_prove_agg(
            &self.app_params_path,
            &self.agg_params_path,
            app_circuit_config_path,
            agg_circuit_config_path,
            &email_path,
            app_pk_path,
            agg_pk_path,
            &acc_path,
            &proof_path,
            &public_input_path,
        )
        .await?;
        Ok(())
    }

    fn nonce2pathes(&self, nonce: usize) -> (String, String, String, String) {
        (
            format!("{}/email_{}.eml", &self.email_dir, nonce),
            format!("{}/acc_{}.hex", &self.email_dir, nonce),
            format!("{}/proof_{}.hex", &self.email_dir, nonce),
            format!("{}/public_input_{}.json", &self.email_dir, nonce),
        )
    }
}
