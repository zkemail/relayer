use ethers_core::types::{Address, U256};
// use ethers_core::utils::CompiledContract;
// use ethers_providers::{Http, Middleware, Provider};
// use ethers_signers::{LocalWallet, Signer};
use dotenv::dotenv;
use ethers::abi::Abi;
use ethers::core::types::TransactionRequest;
use ethers::prelude::*;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
// use hex;
use hex_literal::hex;
use k256::ecdsa::SigningKey;
use rand::thread_rng;
use serde_json::Value;
use std::convert::TryFrom;
use std::error::Error;
use std::fs;
use std::str::{self, FromStr};

#[derive(Debug, Clone)]
struct CircomCalldata {
    pi_a: [U256; 2],
    pi_b: [[U256; 2]; 2],
    pi_c: [U256; 2],
    signals: [U256; 34],
}

// Define a new function that takes optional arguments and provides default values
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Provide default values if arguments are not specified
    let dir = "";
    let nonce = "17689783363368087877";

    // Call the main function with the specified or default values
    let calldata = get_calldata(Some(dir), Some(nonce)).unwrap();
    println!("Calldata: {:?}", calldata);

    // Call the main function with the specified or default values
    match send_to_chain(true, "./data", nonce).await {
        Ok(_) => {
            println!("Successfully sent to chain.");
        }
        Err(err) => {
            eprintln!("Error to send to chain at {}: {}", line!(), err);
        }
    }
    Ok(())
}

// Define a new function that takes optional arguments and provides default values
fn get_calldata(dir: Option<&str>, nonce: Option<&str>) -> Result<CircomCalldata, Box<dyn Error>> {
    // Provide default values if arguments are not specified
    let dir = dir.unwrap_or("");
    let nonce = nonce.unwrap_or("17689783363368087877");

    // Call the main function with the specified or default values
    parse_files_into_calldata(dir, nonce)
}

// #[tokio::main]
// async
fn parse_files_into_calldata(
    dir: &str,
    nonce: &str,
) -> Result<CircomCalldata, Box<dyn std::error::Error>> {
    let proof_json: Value = serde_json::from_str(
        &fs::read_to_string(dir.to_owned() + "rapidsnark_proof_" + nonce + ".json").unwrap(),
    )
    .unwrap();
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
    let private_key_hex =
        std::env::var("PRIVATE_KEY").expect("The PRIVATE_KEY environment variable must be set");
    let rpcurl = if test {
        "http://localhost:8548".to_string()
    } else {
        format!("https://eth-goerli.alchemyapi.io/v2/{}", alchemy_api_key)
    };

    let provider = Provider::<Http>::try_from(rpcurl)?;
    let wallet = if test {
        // Wallet from randomness
        let mut rng = thread_rng();
        LocalWallet::new(&mut rng)
    } else {
        LocalWallet::from_str(&private_key_hex)?
    };

    println!("Wallet address: {}", wallet.address());

    // Read proof and public parameters from JSON files
    let calldata = get_calldata(Some(dir), Some(nonce)).unwrap();

    // Read the contents of the ABI file as bytes
    let abi_bytes = include_bytes!("../data/wallet.abi");
    // Convert the bytes to a string
    let abi_str = str::from_utf8(abi_bytes)?;
    // Parse the string as JSON to obtain the ABI
    let abi_json: Value = serde_json::from_str(abi_str)?;
    // Convert the JSON value to the Abi type
    let abi: Abi = serde_json::from_value(abi_json)?;

    let contract = Contract::new(contract_address, abi, provider.into());

    println!("Sending transaction...");

    // Call the transfer function
    let call = contract.method::<_, ()>(
        "transfer",
        (
            calldata.pi_a,
            calldata.pi_b,
            calldata.pi_c,
            calldata.signals,
        ),
    )?;
    println!("Calling contract fn: {:?}", call);
    let pending_tx = call.send().await?;
    println!("Transaction hash: {:?}", pending_tx);

    Ok(())
}
