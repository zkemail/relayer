use crate::config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, SMTP_PORT_KEY, ZK_EMAIL_PATH_KEY,
};
use crate::imap_client::{EmailReceiver, IMAPAuth};
use crate::parse_email::*;
use crate::smtp_client::EmailSenderClient;
use crate::strings::{first_reply, invalid_reply};
use anyhow::{anyhow, Result};
use dotenv::dotenv;
use http::StatusCode;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
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

pub async fn handle_email(raw_email: String, zk_email_circom_dir: &String) -> Result<()> {
    // Path 1: Write raw_email to ../wallet_{hash}.eml
    let hash = {
        let mut hasher = DefaultHasher::new();
        raw_email.hash(&mut hasher);
        hasher.finish()
    };

    let file_path = format!("{}/wallet_{}.eml", "./received_eml", hash);
    match fs::write(file_path.clone(), raw_email.clone()) {
        Ok(_) => println!("Email data written successfully to {}", file_path),
        Err(e) => println!("Error writing data to file: {}", e),
    }
    std::thread::sleep(std::time::Duration::from_secs(3));
    Ok(())
    // println!("Response status: {}", response.status());
}

pub async fn validate_email(raw_email: &str, emailer: &EmailSenderClient) {
    let mut subject = extract_subject(&raw_email).unwrap();
    let mut from = extract_from(&raw_email).unwrap();
    println!("Subject, from: {:?} {:?}", subject, from);

    // Validate subject, and send rejection/reformatting email if necessary
    let re =
        Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc|dai|ETH|USDC|DAI) to (.+@.+(\..+)+)")
            .unwrap();
    let subject_regex = re.clone();
    let mut custom_reply: String = "".to_string();
    if subject_regex.is_match(subject.as_str()) {
        if let Some(captures) = re.captures(subject.as_str()) {
            // Extract the amount and recipient from the captures
            let amount = captures.get(1).map_or("", |m| m.as_str());
            let recipient = captures.get(4).map_or("", |m| m.as_str());
            custom_reply = first_reply(amount, recipient);
        } else {
            custom_reply = invalid_reply("seems to match regex but is invalid");
        }
        println!("Send valid! Validating proof...");
    } else {
        println!("Send invalid! Regex failed...");
        custom_reply = invalid_reply("failed regex");
    }
    let confirmation = emailer.reply_all(raw_email, &custom_reply);
}
