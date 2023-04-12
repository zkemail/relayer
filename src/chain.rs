use ethers_core::types::{Address, U256};
// use ethers_core::utils::CompiledContract;
use ethers_providers::{Http, Middleware, Provider};
use ethers_signers::{LocalWallet, Signer};
use rand::thread_rng;
use ethers::prelude::*;
use serde_json::Value;
use std::convert::TryFrom;
use std::error::Error;
use std::fs;

#[derive(Debug, Clone)]
struct CircomCalldata {
    pi_a: [U256; 2],
    pi_b: [[U256; 2]; 2],
    pi_c: [U256; 2],
    public: Vec<U256>,
}

// Define a new function that takes optional arguments and provides default values
fn main() -> Result<(), Box<dyn Error>> {
    // Provide default values if arguments are not specified
    let dir = "";
    let nonce = "17689783363368087877";

    // Call the main function with the specified or default values
    let calldata = get_calldata(Some(dir), Some(nonce)).unwrap();
    println!("Calldata: {:?}", calldata);
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

    let pi_b: [[U256; 2]; 2] = [
        [
            U256::from_dec_str(proof_json["pi_b"][0][0].as_str().unwrap()).unwrap(),
            U256::from_dec_str(proof_json["pi_b"][0][1].as_str().unwrap()).unwrap(),
        ],
        [
            U256::from_dec_str(proof_json["pi_b"][1][0].as_str().unwrap()).unwrap(),
            U256::from_dec_str(proof_json["pi_b"][1][1].as_str().unwrap()).unwrap(),
        ],
    ];

    let pi_c: [U256; 2] = [
        U256::from_dec_str(proof_json["pi_c"][0].as_str().unwrap()).unwrap(),
        U256::from_dec_str(proof_json["pi_c"][1].as_str().unwrap()).unwrap(),
    ];

    let public: Vec<U256> = public_json
        .as_array()
        .unwrap()
        .iter()
        .map(|x| U256::from_dec_str(x.as_str().unwrap()).unwrap())
        .collect();

    let calldata = CircomCalldata {
        pi_a,
        pi_b,
        pi_c,
        public,
    };
    Ok(calldata)
}

// local: bool - whether or not to send to a local RPC
async fn send_to_chain(test: bool) -> Result<(), Box<dyn std::error::Error>> {
    let alchemy_api_key = std::env::var("ALCHEMY_API_KEY").unwrap();
    let contract_address = std::env::var("CONTRACT_ADDRESS").unwrap();
    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let msg_len = 26; // Update this to the appropriate length
    let rpcurl = if test {
        "localhost:8547".to_string()
    } else {
        format!("https://eth-goerli.alchemyapi.io/v2/{}", alchemy_api_key)
    };

    let provider = Provider::<Http>::try_from(rpcurl)?;
    let wallet = if test {
        // Wallet from randomness
        let mut rng = thread_rng();
        let wallet = LocalWallet::new(&mut rng);
        let wallet = wallet.connect(provider)
    } else {
        LocalWallet::new(&private_key, provider)
    };

    // Read proof and public parameters from JSON files
    let calldata = get_calldata(Some(dir), Some(nonce)).unwrap();

    // TODO: Foundry export abi
    let contract = Contract::from_json(
        wallet,
        contract_address.parse()?,
        include_bytes!("../../zk-email-verify/src/contracts/wallet.abi"),
    )?;

    // Call the transfer function
    let tx = contract
        .call(
            "transfer",
            (
                calldata.pi_a,
                calldata.pi_b,
                calldata.pi_c,
                calldata.public,
            ),
            None,
            Options::default(),
        )
        .await?;

    println!("Transaction hash: {:?}", tx);

    Ok(())
}
