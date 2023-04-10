use ethers::prelude::*;
use ethers::providers::Provider;
use serde_json::Value;
use std::convert::TryFrom;
use std::fs;

#[tokio::main]
async fn send_to_chain() -> Result<(), Box<dyn std::error::Error>> {
    let alchemy_api_key = std::env::var("ALCHEMY_API_KEY").unwrap();
    let contract_address = std::env::var("CONTRACT_ADDRESS").unwrap();
    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let msg_len = 26; // Update this to the appropriate length

    let provider = Provider::try_from(format!(
        "https://eth-goerli.alchemyapi.io/v2/{}",
        alchemy_api_key
    ))?;
    let wallet = Wallet::from_private_key(private_key, provider).await?;

    // Read proof and public parameters from JSON files
    let proof_json: Value = serde_json::from_str(&fs::read_to_string(
        "build/email/rapidsnark_proof_17689783363368087877.json",
    )?)?;
    let public_json: Value = serde_json::from_str(&fs::read_to_string(
        "build/email/rapidsnark_public_17689783363368087877.json",
    )?)?;

    let pi_a: [U256; 2] = [
        U256::from_dec_str(proof_json["pi_a"][0].as_str().unwrap())?,
        U256::from_dec_str(proof_json["pi_a"][1].as_str().unwrap())?,
    ];

    let pi_b: [[U256; 2]; 2] = [
        [
            U256::from_dec_str(proof_json["pi_b"][0][0].as_str().unwrap())?,
            U256::from_dec_str(proof_json["pi_b"][0][1].as_str().unwrap())?,
        ],
        [
            U256::from_dec_str(proof_json["pi_b"][1][0].as_str().unwrap())?,
            U256::from_dec_str(proof_json["pi_b"][1][1].as_str().unwrap())?,
        ],
    ];

    let pi_c: [U256; 2] = [
        U256::from_dec_str(proof_json["pi_c"][0].as_str().unwrap())?,
        U256::from_dec_str(proof_json["pi_c"][1].as_str().unwrap())?,
    ];

    let signals: Vec<U256> = public_json
        .as_array()
        .unwrap()
        .iter()
        .map(|x| U256::from_dec_str(x.as_str().unwrap()).unwrap())
        .collect();

    let contract = Contract::from_json(
        wallet,
        contract_address.parse()?,
        include_bytes!("../path/to/your/contract/abi.json"),
    )?;

    // Call the transfer function
    let tx = contract
        .call(
            "transfer",
            (pi_a, pi_b, pi_c, signals),
            None,
            Options::default(),
        )
        .await?;

    println!("Transaction hash: {:?}", tx);

    Ok(())
}
