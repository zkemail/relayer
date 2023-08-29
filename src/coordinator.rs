use crate::chain::query_address;
use crate::db::get_or_store_salt;
use crate::parse_email::*;
use crate::smtp_client::EmailSenderClient;
use crate::strings::*;
use anyhow::{anyhow, Result};
use ark_bn254::Fr;
use arkworks_mimc::params::round_keys_contants_to_vec;
use arkworks_mimc::{
    params::mimc_5_220_bn254::{MIMC_5_220_BN254_PARAMS, MIMC_5_220_BN254_ROUND_KEYS},
    MiMC,
};
use http::StatusCode;
use num_bigint::BigUint;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    fs,
    hash::{Hash, Hasher},
};

#[derive(Debug, Deserialize, Serialize)]
struct EmailEvent {
    dkim: Option<String>,
    subject: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

pub struct BalanceRequest {
    pub address: String,
    pub amount: String,
    pub token_name: String,
}

/// Pending means we are monitoring the blockchain for a transaction to fill the wallet
/// Ready means we have a transaction that has filled the wallet and we sent the tx and reply
/// Failure means we have a transaction that has filled the wallet but we failed to send the tx and reply properly
/// Unvalidated means we just saw the email and haven't processed it yet
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ValidationStatus {
    Ready,
    Failure,
    Pending,
    Unvalidated,
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
        "https://ziztuww--aayush-pull-and-prove-email.modal.run?aws_url={}&nonce={}",
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

pub fn calculate_hash(raw_email: &String) -> String {
    let mut hasher = DefaultHasher::new();
    raw_email.hash(&mut hasher);
    hasher.finish().to_string()
}

pub async fn handle_email(
    raw_email: String,
    zk_email_circom_dir: &str,
    nonce: Option<String>,
) -> Result<()> {
    // Path 1: Write raw_email to ../wallet_{nonce}.eml
    // This nonce is usually (from_message_id)_(to_message_id)_(hash), but absent of that is the hash
    let file_id = match nonce {
        Some(s) => s,
        None => calculate_hash(&raw_email),
    };

    let file_path = format!("{}/wallet_{}.eml", "./received_eml", file_id);
    match fs::write(file_path.clone(), raw_email.clone()) {
        Ok(_) => println!("Email data written successfully to {}", file_path),
        Err(e) => println!("Error writing data to file: {}", e),
    }
    Ok(())
}

pub async fn calculate_decimal_salt(email_address: &str, message_id: &str) -> Result<String> {
    let mimc = MiMC::<Fr, MIMC_5_220_BN254_PARAMS>::new(
        1,
        Fr::from(123),
        round_keys_contants_to_vec(&MIMC_5_220_BN254_ROUND_KEYS),
    );

    let email_arr = email_address.as_bytes();
    let message_id_arr = message_id.as_bytes();

    const MAX_EMAIL_LEN: usize = 31;
    const MAX_MESSAGE_ID_LEN: usize = 128;
    let mut email_arr_32 = [0u8; MAX_EMAIL_LEN];
    let mut message_id_arr_32 = [0u8; MAX_MESSAGE_ID_LEN];
    if email_arr.len() > MAX_EMAIL_LEN || message_id_arr.len() > MAX_MESSAGE_ID_LEN {
        return Err(anyhow!(
            "Either length of email or message_id is more than the max"
        ));
    }

    email_arr_32[..email_arr.len()].copy_from_slice(email_arr);
    message_id_arr_32[..message_id_arr.len()].copy_from_slice(message_id_arr);

    let mut input_arr = [0u8; MAX_EMAIL_LEN + MAX_MESSAGE_ID_LEN];
    input_arr[..MAX_EMAIL_LEN].copy_from_slice(&email_arr_32);
    input_arr[MAX_EMAIL_LEN..].copy_from_slice(&message_id_arr_32);
    println!("input_arr: {:?}", input_arr);
    let input_vec = input_arr.map(Fr::from).to_vec();
    let create2_salt_fp256 = mimc.permute_feistel(input_vec)[0];
    println!("create2_salt_fp256: {}", create2_salt_fp256);
    // Assuming you have an Fp256 value called `fp_value`
    let create2_salt_value: BigUint = create2_salt_fp256.into();

    // To print the value in decimal
    let decimal_salt = create2_salt_value.to_str_radix(10);
    println!("Decimal create2 salt: {}", decimal_salt);

    Ok(decimal_salt)
}

pub async fn calculate_address(email_address: &str, message_id: &str) -> Result<String> {
    let decimal_salt = calculate_decimal_salt(email_address, message_id).await?;
    let address_raw = query_address(false, decimal_salt.as_str()).await?;
    let address = format!("0x{:x}", address_raw);
    Ok(address)
}

// Note: This function often mis-infers things, and gives weird subjects like "Subject:To;"
pub async fn validate_email_infer(
    raw_email: &str,
    emailer: &EmailSenderClient,
    send_reply: Option<bool>,
) -> Result<(
    ValidationStatus,
    Option<String>,
    Option<String>,
    Option<BalanceRequest>,
)> {
    let from = extract_from(raw_email).unwrap_or("".to_string());
    let subject = extract_subject(raw_email).unwrap_or("".to_string());
    validate_email_envelope(raw_email, emailer, "From", "Subject", send_reply).await
}

/// This function validates the email envelope by checking the subject and sender of the email.
/// It uses regular expressions to match the subject to a specific pattern and extracts the necessary information.
/// If the subject matches the pattern, it calculates the sender and recipient addresses and checks the sender's balance.
/// Depending on the validation status, it sends a reply email and returns the validation status, sender salt, recipient salt, and balance request.
pub async fn validate_email_envelope(
    raw_email: &str,
    emailer: &EmailSenderClient,
    from_str: &str,
    subject_str: &str,
    send_reply: Option<bool>,
) -> Result<(
    ValidationStatus,
    Option<String>,
    Option<String>,
    Option<BalanceRequest>,
)> {
    let from = from_str.to_string();
    let subject = subject_str.to_string();
    let send_reply = send_reply.unwrap_or(true);
    let custom_reply;

    // Validate subject, and send rejection/reformatting email if necessary
    let result = parse_subject_for_send(subject.as_str());
    let (amount, currency, recipient) = match result {
        Ok((amt, cur, rec)) => (amt, cur, rec),
        Err(_) => {
            custom_reply = invalid_reply();
            if send_reply {
                send_confirmation_email(raw_email, &custom_reply, emailer).await;
            }
            return Ok((ValidationStatus::Failure, None, None, None));
        }
    };

    let message_id_unwrapped = match extract_message_id(raw_email) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    let message_id = match message_id_unwrapped {
        Some(id) => id,
        None => {
            custom_reply = bad_message_id();
            if send_reply {
                send_confirmation_email(raw_email, &custom_reply, emailer).await;
            }
            return Ok((ValidationStatus::Failure, None, None, None));
        }
    };

    println!(
        "Subject, from, message id: {:?} {:?} {:?}",
        subject, from, message_id
    );

    let (sender_salt_exists, sender_salt_raw) =
        get_or_store_salt(from.as_str(), message_id.as_str())
            .await
            .unwrap();
    let (recipient_salt_exists, recipient_salt_raw) =
        get_or_store_salt(recipient.as_str(), message_id.as_str())
            .await
            .unwrap();
    let sender_salt = Some(sender_salt_raw.clone());
    let recipient_salt = Some(recipient_salt_raw.clone());
    let sender_address = calculate_address(from.as_str(), sender_salt_raw.as_str())
        .await
        .unwrap();

    let recipient_address = calculate_address(recipient.as_str(), recipient_salt_raw.as_str())
        .await
        .unwrap();

    custom_reply = pending_reply(
        sender_address.clone().as_str(),
        &amount,
        &currency,
        &recipient,
    )
    .await;
    let valid = ValidationStatus::Pending;

    let balance_request = Some(BalanceRequest {
        address: sender_address,
        amount: amount.to_string(),
        token_name: currency.to_string(),
    });

    if ValidationStatus::Ready == valid {
        println!("Send valid! Validating proof...");
    } else if valid == ValidationStatus::Pending {
        println!("Send valid, waiting for funds...");
    } else {
        println!("Send invalid! Regex failed...");
    }

    if send_reply {
        send_confirmation_email(raw_email, &custom_reply, emailer).await;
    }

    Ok((valid, sender_salt, recipient_salt, balance_request))
}

async fn send_confirmation_email(raw_email: &str, custom_reply: &str, emailer: &EmailSenderClient) {
    let confirmation: std::result::Result<(), Box<dyn Error>> =
        emailer.reply_all(raw_email, custom_reply, false);
    match confirmation {
        Ok(_) => println!("Confirmation email sent successfully."),
        Err(e) => println!("Error sending confirmation email: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculate_address() -> Result<()> {
        let email_address = "aayushgupta05@gmail.com";
        let message_id = "CA+OJ5QeHStpKMCy9fcpCaLe8TaKTomRRG0SzNJUYevZ=2QS=PA@mail.gmail.com";
        let result = calculate_decimal_salt(email_address, message_id).await;
        match result {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Failed to calculate decimal_salt: {}", e)),
        };
        let decimal_salt = result.unwrap();
        assert!(!decimal_salt.is_empty(), "decimal_salt is empty");
        println!("decimal_salt: {}", decimal_salt);
        assert!(
            decimal_salt
                == "11578046119786885486589898473893761816011340408005885677852497807442621066251",
            "Decimal salt is incorrect"
        );

        let result_address = calculate_address(email_address, message_id).await;
        match result_address {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Failed to calculate address: {}", e)),
        };
        let address = result_address.unwrap();
        assert!(!address.is_empty(), "address is empty");
        println!("address: {}", address);
        assert!(
            address == "0x93b3c87c76c8a9e580e5cbf58fa20e579e76414e",
            "Address is incorrect"
        );
        Ok(())
    }
}
