use ethers_core::types::{Address, U256};
// use ethers_core::utils::CompiledContract;
// use ethers_providers::{Http, Middleware, Provider};
// use ethers_signers::{LocalWallet, Signer};
mod config;
mod imap_client;
mod parse_email;
mod processer;
mod smtp_client;

use dotenv::dotenv;
use ethers::abi::Abi;
use ethers::contract::ContractError;
use ethers::prelude::*;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
// use hex;
use crate::config::{INCOMING_EML_PATH, LOGIN_ID_KEY, LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY};
use crate::smtp_client::EmailSenderClient;
use hex_literal::hex;
use k256::ecdsa::SigningKey;
use rand::thread_rng;
use serde_json::Value;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::fs;
use std::str::{self, FromStr};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct CircomCalldata {
    pi_a: [U256; 2],
    pi_b: [[U256; 2]; 2],
    pi_c: [U256; 2],
    signals: [U256; 34],
}

// Call like: cargo run --bin chain -- <proof_outputs_dir> <nonce>
// Define a new function that takes optional arguments and provides default values
// Running with test=true overrides the RPC URL to default to localhost no matter what
// TODO: replace test=true with rpc url instead
async fn run(test: bool, dir: &str, nonce: &str) -> Result<(), Box<dyn Error>> {
    // Call the main function with the specified or default values
    let calldata = get_calldata(Some(dir), Some(nonce)).unwrap();
    println!("Calldata: {:?}", calldata);

    // Call the main function with the specified or default values
    match send_to_chain(test, dir, nonce).await {
        Ok(_) => {
            println!("Successfully sent to chain.");
        }
        Err(err) => {
            eprintln!("Error sending to chain: {}", err);
        }
    };
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // Provide default values if arguments are not specified
    let dir = args.get(1).map_or("", String::as_str);
    let nonce = args.get(2).map_or("", String::as_str);

    // Call the run function with the specified or default values
    run(false, dir, nonce).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_with_defaults() {
        let dir = "";
        let nonce = "17689783363368087877";

        println!("Make sure anvil is running on localhost:8548.");

        // Call the run function with default values in the test
        let result = run(true, dir, nonce).await;
        assert!(result.is_ok());
    }
}

// Define a new function that takes optional arguments and provides default values
fn get_calldata(dir: Option<&str>, nonce: Option<&str>) -> Result<CircomCalldata, Box<dyn Error>> {
    // Provide default values if arguments are not specified
    let dir = dir.unwrap_or("");
    let nonce = nonce.unwrap_or("");

    // Call the main function with the specified or default values
    parse_files_into_calldata(dir, nonce)
}

// #[tokio::main]
// async
fn parse_files_into_calldata(
    dir: &str,
    nonce: &str,
) -> Result<CircomCalldata, Box<dyn std::error::Error>> {
    let proof_dir = dir.to_owned() + "rapidsnark_proof_" + nonce + ".json";
    let proof_json: Value = serde_json::from_str(&fs::read_to_string(proof_dir).unwrap()).unwrap();
    let public_json: Value = serde_json::from_str(
        &fs::read_to_string(dir.to_owned() + "rapidsnark_public_" + nonce + ".json").unwrap(),
    )
    .unwrap();

    let pi_a: [U256; 2] = [
        U256::from_dec_str(proof_json["pi_a"][0].as_str().unwrap()).unwrap(),
        U256::from_dec_str(proof_json["pi_a"][1].as_str().unwrap()).unwrap(),
    ];

    let pi_b_raw: [[U256; 2]; 2] = [
        [
            U256::from_dec_str(proof_json["pi_b"][0][0].as_str().unwrap()).unwrap(),
            U256::from_dec_str(proof_json["pi_b"][0][1].as_str().unwrap()).unwrap(),
        ],
        [
            U256::from_dec_str(proof_json["pi_b"][1][0].as_str().unwrap()).unwrap(),
            U256::from_dec_str(proof_json["pi_b"][1][1].as_str().unwrap()).unwrap(),
        ],
    ];

    // Swap the G2 points to be the correct order with the new snarkjs
    let pi_b_swapped: Vec<[U256; 2]> = pi_b_raw.iter().map(|inner| [inner[1], inner[0]]).collect();

    // Convert the Vec to an array
    let pi_b: [[U256; 2]; 2] = [pi_b_swapped[0], pi_b_swapped[1]];

    let pi_c: [U256; 2] = [
        U256::from_dec_str(proof_json["pi_c"][0].as_str().unwrap()).unwrap(),
        U256::from_dec_str(proof_json["pi_c"][1].as_str().unwrap()).unwrap(),
    ];

    let signals: [U256; 34] = public_json
        .as_array()
        .unwrap()
        .iter()
        .map(|x| U256::from_dec_str(x.as_str().unwrap()).unwrap())
        .collect::<Vec<_>>()
        .as_slice()
        .try_into()
        .unwrap();

    let calldata = CircomCalldata {
        pi_a,
        pi_b,
        pi_c,
        signals,
    };
    Ok(calldata)
}

// local: bool - whether or not to send to a local RPC
// dir: data directory where the intermediate rapidsnark inputs/proofs will be stored
pub async fn send_to_chain(
    test: bool,
    dir: &str,
    nonce: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from the .env file
    dotenv().ok();

    let alchemy_api_key = std::env::var("ALCHEMY_GOERLI_KEY").unwrap();
    let contract_address: Address = std::env::var("CONTRACT_ADDRESS").unwrap().parse()?;

    // Get the private key from the environment variable
    println!("alchemy_api_key: {}", alchemy_api_key);
    let private_key_hex =
        std::env::var("PRIVATE_KEY").expect("The PRIVATE_KEY environment variable must be set");
    let rpcurl = if test {
        "http://localhost:8548".to_string()
    } else {
        std::env::var("RPC_URL").expect("The RPC_URL environment variable must be set")
    };
    println!("rpcurl: {}", rpcurl);

    let provider = Provider::<Http>::try_from(rpcurl)?;
    let wallet = LocalWallet::from_str(&private_key_hex)?;

    println!("Wallet address: {}", wallet.address());

    // Read proof and public parameters from JSON files
    let calldata = get_calldata(Some(dir), Some(nonce)).unwrap();

    // Read the contents of the ABI file as bytes
    let abi_bytes = include_bytes!("../abi/wallet.abi");
    // Convert the bytes to a string
    let abi_str = str::from_utf8(abi_bytes)?;
    // Parse the string as JSON to obtain the ABI
    let abi_json: Value = serde_json::from_str(abi_str)?;
    // Convert the JSON value to the Abi type
    let abi: Abi = serde_json::from_value(abi_json)?;

    println!("Provider: {:?}", provider);
    // TODO: Hardcoded chain id
    let chain_id: u64 = 5;
    let gas_price = provider.get_gas_price().await?;
    let signer = SignerMiddleware::new(provider, wallet.with_chain_id(chain_id));
    let contract = ContractInstance::new(contract_address, abi, signer);

    println!("Sending transaction with gas price {:?}...", gas_price);

    // Call the transfer function
    let call = contract
        .method::<_, ()>(
            "transfer",
            (
                calldata.pi_a,
                calldata.pi_b,
                calldata.pi_c,
                calldata.signals,
            ),
        )?
        .gas_price(gas_price); // Set an appropriate gas limit

    println!("Calling call: {:?}", call);

    let pending_tx = match call.send().await {
        Ok(tx) => tx,
        Err(e) => {
            println!("Error: {:?}", e);
            reply_with_message(nonce, "Error sending transaction. Most likely your email domain is not supported (must be @gmail.com, @hotmail.com, @ethereum.org, or @skiff.com).");
            println!("Error bytes: {:?}", e.as_revert());
            return Err(e.into());
        }
    };
    println!("Transaction hash: {:?}", pending_tx);
    reply_with_etherscan(nonce, pending_tx.tx_hash());
    Ok(())
}

fn reply_with_etherscan(nonce: &str, tx_hash: H256) {
    let etherscan_url = format!("https://goerli.etherscan.io/tx/0x{:x}", tx_hash);
    let reply = format!(
        "Transaction sent! View Etherscan confirmation: {}.",
        etherscan_url
    );
    println!("Replying with confirmation...{}", reply);
    reply_with_message(nonce, &reply);
}

fn reply_with_message(nonce: &str, reply: &str) {
    dotenv().ok();
    let mut sender: EmailSenderClient = EmailSenderClient::new(
        env::var(LOGIN_ID_KEY).unwrap().as_str(),
        env::var(LOGIN_PASSWORD_KEY).unwrap().as_str(),
        Some(env::var(SMTP_DOMAIN_NAME_KEY).unwrap().as_str()),
    );
    // Read raw email from received_eml/wallet_{nonce}.eml
    let eml_var = env::var(INCOMING_EML_PATH).unwrap();

    let raw_email = fs::read_to_string(format!("{}/wallet_{}.eml", eml_var, nonce)).unwrap();
    let confirmation = sender.reply_all(&raw_email, &reply);
}
