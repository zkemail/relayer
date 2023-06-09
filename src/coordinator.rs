use crate::config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, SMTP_PORT_KEY, ZK_EMAIL_PATH_KEY,
};
use crate::chain::{get_token_balance};
use crate::imap_client::{EmailReceiver, IMAPAuth};
use crate::parse_email::*;
use crate::smtp_client::EmailSenderClient;
use crate::strings::{first_reply, invalid_reply, pending_reply};
use anyhow::{anyhow, Result};
use arkworks_mimc::{
    params::{
        mimc_5_220_bn254::{MIMC_5_220_BN254_PARAMS, MIMC_5_220_BN254_ROUND_KEYS},
        round_keys_contants_to_vec,
    },  
    MiMC, MiMCParameters
};
use ark_ff::{fields::Fp256, PrimeField};
use ark_bn254::{Bn254, FrParameters, Fr};
use dotenv::dotenv;
use http::StatusCode;
use lettre::message;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use sled;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::string;
use std::{
    collections::hash_map::DefaultHasher,
    env,
    error::Error,
    fs,
    hash::{Hash, Hasher},
};
#[derive(Debug, Deserialize)]
struct EmailEvent {
    dkim: Option<String>,
    subject: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

pub struct BalanceRequest {
    pub address: String,
    pub amount: String,
    pub token_name: String
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ValidationStatus {
    Ready,
    Failure,
    Pending,
}

// Dummy future that does nothing
struct DummyFuture;

impl Future for DummyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

// Used for Rust to automatically send to modal
// However, I use the python coordinator for now since
// this code needs to add aws s3 upload code to reach parity with that.
pub async fn send_to_modal(raw_email: String, hash: u64) -> Result<()> {
    // Path 2: Send to modal
    // Construct the URL with query parameters
    let webhook_url = format!(
        "https://ziztuww--aayush-test.modal.run?aws_url={}&nonce={}",
        urlencoding::encode(&raw_email),
        hash
    );

    // Create a new reqwest client
    let client = Client::new();

    // Send the POST request
    let response_result: Result<reqwest::Response, reqwest::Error> = client
        .post(&webhook_url)
        .header("Content-Type", "application/octet-stream")
        .body(raw_email)
        .send()
        .await;
    let response = response_result?;

    // Check the status code of the response
    match response.status() {
        StatusCode::OK => {
            // Read the response body
            let response_body = response.text().await?;
            // Handle the successful response (e.g., print the response body)
            println!("Modal response: {}", response_body);
        }
        StatusCode::BAD_REQUEST => {
            // Handle the bad request error (e.g., print an error message)
            println!("Bad request to Modal");
        }
        _ => {
            // Handle other status codes (e.g., print a generic error message)
            println!("An error occurred on Modal...");
        }
    };
    Ok(())
}

pub async fn handle_email(raw_email: String, zk_email_circom_dir: &String, nonce: Option<String>) -> Result<()> {
    // Path 1: Write raw_email to ../wallet_{hash}.eml
    
    let file_id = match nonce {
        Some(s) => s,
        None => {
            let mut hasher = DefaultHasher::new();
            raw_email.hash(&mut hasher);
            hasher.finish().to_string()
        }
    };

    let file_path = format!("{}/wallet_{}.eml", "./received_eml", file_id);
    match fs::write(file_path.clone(), raw_email.clone()) {
        Ok(_) => println!("Email data written successfully to {}", file_path),
        Err(e) => println!("Error writing data to file: {}", e),
    }
    std::thread::sleep(std::time::Duration::from_secs(3));
    Ok(())
    // println!("Response status: {}", response.status());
}

/// This function retrieves the salt associated with an email address and message ID.
/// If the email exists in the database, it returns true and the salt as a string.
/// If the email is not found, it stores the message id and returns false and that as the salt string.
pub async fn get_salt(email: &str, message_id: &str) -> Result<(bool, String)> {
    let db = match sled::open("./db/email_to_salt") {
        Ok(database) => database,
        Err(e) => return Err(anyhow!("Failed to open database: {}", e)),
    };
    let email_exists = db.get(email)?;
    if let Some(salt) = email_exists {
        Ok((true, std::str::from_utf8(&salt)?.to_string()))
    } else {
        db.insert(email, message_id)?;
        Ok((false, message_id.to_string()))
    }
}

// fn bytes_to_fp256_vec(input_arr: &[u8]) -> Vec<Fp256<FrParameters>> {
//     let input_vec: Vec<Fr> = input_arr
//     .chunks(32)
//     .map(|chunk| {
//         let mut arr = [0u8; 32];
//         arr.copy_from_slice(chunk);
//         Fr::from_repr(Fp256::<FrParameters>::from_repr(arr.into()).into_repr())
//     })
//     .collect();

//     // input
//     //     .chunks(32)
//     //     .map(|chunk| {
//     //         let mut arr = [0u8; 32];
//     //         arr.copy_from_slice(chunk);
//     //         Fp256::<FrParameters>::from_repr(arr.into())
//     //     })
//     //     .collect()
// }

// TODO FUNCTION
pub fn calculate_address(email: &str, message_id: &str) -> Result<String> {
    let mimc = MiMC::<Fr, MIMC_5_220_BN254_PARAMS>::new(
        1,
        Fr::from(123),
        round_keys_contants_to_vec(&MIMC_5_220_BN254_ROUND_KEYS),
    );

    let email_arr = email.as_bytes();
    let message_id_arr = message_id.as_bytes();
    const MAX_EMAIL_LEN: usize = 31;
    const MAX_MESSAGE_ID_LEN: usize = 128;
    let mut email_arr_32 = [0u8; MAX_EMAIL_LEN];
    let mut message_id_arr_32 = [0u8; MAX_MESSAGE_ID_LEN];
    if email_arr.len() > MAX_EMAIL_LEN || message_id_arr.len() > MAX_MESSAGE_ID_LEN {
        return Err(anyhow!("Either length of email or message_id is more than the max"));
    }
    for i in 0..email_arr.len() {
        email_arr_32[i] = email_arr[i];
    }
    for i in 0..message_id_arr.len() {
        message_id_arr_32[i] = message_id_arr[i];
    }
    let mut input_arr = [0u8; MAX_EMAIL_LEN + MAX_MESSAGE_ID_LEN];
    input_arr[..MAX_EMAIL_LEN].copy_from_slice(&email_arr_32);
    input_arr[MAX_EMAIL_LEN..].copy_from_slice(&message_id_arr_32);
    let input_vec = input_arr.map(|x| Fr::from(x)).to_vec();
    let result = mimc.permute_feistel(input_vec);
    let address = format!("{:?}", result);
    Ok(address)
}

pub async fn validate_email(raw_email: &str, emailer: &EmailSenderClient) -> Result<(ValidationStatus, Option<String>, Option<String>, Option<BalanceRequest>)> {
    let subject = extract_subject(&raw_email).unwrap();
    
    // Validate subject, and send rejection/reformatting email if necessary
    let re = Regex::new(
        r"([Ss]end|[Tt]ransfer) ?\$?(\d+(\.\d+)?) (eth|usdc|dai|test|ETH|USDC|DAI|TEST|Dai|Eth|Usdc|Test) to (.+@.+(\..+)+)",
    )
    .unwrap();
    let subject_regex = re.clone();
    let from = extract_from(&raw_email).unwrap();
    let message_id = extract_message_id(&raw_email).unwrap();
    println!(
        "Subject, from, message id: {:?} {:?} {:?}",
        subject, from, message_id
    );

    let mut custom_reply: String = "".to_string();
    let mut valid: ValidationStatus = ValidationStatus::Pending;
    let mut sender_salt: Option<String> = None;
    let mut recipient_salt: Option<String> = None;
    let mut sender_address: Option<String> = None; // Included since we want to check its balance before sending
    let mut balance_request: Option<BalanceRequest> = None;
    
    if subject_regex.is_match(subject.as_str()) {
        let regex_subject = subject.clone();
        let captures = re.captures(regex_subject.as_str());
        if let Some(captures) = captures {
            // Extract the amount and recipient from the captures
            let amount = captures.get(2).map_or("", |m| m.as_str());
            let currency = captures.get(4).map_or("", |m| m.as_str());
            let recipient = captures.get(5).map_or("", |m| m.as_str());
            println!(
                "Amount: {}, Recipient: {}, Currency: {}",
                amount, recipient, currency
            );
            
            let (sender_salt_exists, sender_salt_raw) = get_salt(from.as_str(), message_id.as_str()).await.unwrap();
            let (recipient_salt_exists, recipient_salt_raw) = get_salt(recipient, message_id.as_str()).await.unwrap();
            sender_salt = Some(sender_salt_raw.clone());
            recipient_salt = Some(recipient_salt_raw.clone());
            sender_address = Some(calculate_address(from.as_str(), sender_salt_raw.as_str()).unwrap());
            let recipient_address = calculate_address(recipient, recipient_salt_raw.as_str()).unwrap();
            // TODO: Check balance here
            if sender_salt_exists {
                custom_reply = first_reply(amount, currency, recipient);
                valid = ValidationStatus::Ready;
            } else {
                custom_reply = pending_reply(sender_address.clone().unwrap().as_str(), amount, currency, recipient);
                valid = ValidationStatus::Pending;
            }
            balance_request = Some(BalanceRequest {
                address: sender_address.unwrap(),
                amount: amount.to_string(),
                token_name: currency.to_string(),
            });
        } else {
            custom_reply = invalid_reply("seems to match regex but is invalid");
            valid = ValidationStatus::Failure;
        }
    } else {
        custom_reply = invalid_reply("failed regex");
        valid = ValidationStatus::Failure;
    }
    if ValidationStatus::Ready == valid {
        println!("Send valid! Validating proof...");
    } else if valid == ValidationStatus::Pending {  
        println!("Send valid, waiting for funds...");
    } else {
        println!("Send invalid! Regex failed...");
    }
    let confirmation: std::result::Result<(), Box<dyn Error>> = emailer.reply_all(raw_email, &custom_reply);
    match confirmation {
        Ok(_) => println!("Confirmation email sent successfully."),
        Err(e) => println!("Error sending confirmation email: {}", e),
    }

    return Ok((valid, sender_salt, recipient_salt, balance_request));
}
