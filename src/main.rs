// mod chain;
mod config;
mod imap_client;
mod parse_email;
mod processer;
mod smtp_client;
use anyhow::{anyhow, Result};
use config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, SMTP_PORT_KEY, ZK_EMAIL_PATH_KEY,
};
use dotenv::dotenv;
use http::StatusCode;
use imap_client::{EmailReceiver, IMAPAuth};
use parse_email::*;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use smtp_client::EmailSenderClient;
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

async fn handle_email(raw_email: String, zk_email_circom_dir: &String) -> Result<()> {
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

    // Path 2: Send to modal
    // Construct the URL with query parameters
    // let webhook_url = format!(
    //     "https://ziztuww--aayush-test.modal.run?aws_url={}&nonce={}",
    //     urlencoding::encode(&raw_email),
    //     hash
    // );

    // // Create a new reqwest client
    // let client = Client::new();

    // // Send the POST request
    // let response_result: Result<reqwest::Response, reqwest::Error> = client
    //     .post(&webhook_url)
    //     .header("Content-Type", "application/octet-stream")
    //     .body(raw_email)
    //     .send()
    //     .await;
    // let response = response_result?;

    // // Check the status code of the response
    // match response.status() {
    //     StatusCode::OK => {
    //         // Read the response body
    //         let response_body = response.text().await?;
    //         // Handle the successful response (e.g., print the response body)
    //         println!("Modal response: {}", response_body);
    //     }
    //     StatusCode::BAD_REQUEST => {
    //         // Handle the bad request error (e.g., print an error message)
    //         println!("Bad request to Modal");
    //     }
    //     _ => {
    //         // Handle other status codes (e.g., print a generic error message)
    //         println!("An error occurred on Modal...");
    //     }
    // };

    Ok(())
    // println!("Response status: {}", response.status());
}

pub async fn validate_email(raw_email: &str, emailer: &EmailSenderClient) {
    let mut subject = extract_subject(&raw_email).unwrap();
    let mut from = extract_from(&raw_email).unwrap();
    println!("Subject, from: {:?} {:?}", subject, from);

    // Validate subject, and send rejection/reformatting email if necessary
    let re = Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc|dai) to (.+@.+(\..+)+)").unwrap();
    let subject_regex = re.clone();
    let mut custom_reply: String = "".to_string();
    if subject_regex.is_match(subject.as_str()) {
        if let Some(captures) = re.captures(subject.as_str()) {
            // Extract the amount and recipient from the captures
            let amount = captures.get(1).map_or("", |m| m.as_str());
            let recipient = captures.get(4).map_or("", |m| m.as_str());
            custom_reply = format!("Valid send initiated. Sending {} TestERC20 to {} on Ethereum. We will follow up with Etherscan link when finished! You are sending with ", amount, recipient);
        } else {
            custom_reply = "Send seems to match regex but is invalid! Please try again with this subject: \"Send _ eth to __@__.___\"".to_string();
        }
        println!("Send valid! Validating proof...");
    } else {
        println!("Send invalid! Regex failed...");
        custom_reply =
            "Send invalid! Please try again with this subject: \"Send _ eth to __@__.___\""
                .to_string();
    }
    let confirmation = emailer.reply_all(raw_email, &custom_reply);
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let domain_name = env::var(IMAP_DOMAIN_NAME_KEY)?;
    let zk_email_circom_path = env::var(ZK_EMAIL_PATH_KEY)?;
    let port = env::var(IMAP_PORT_KEY)?.parse()?;
    let auth_type = env::var(IMAP_AUTH_TYPE_KEY)?;
    let imap_auth = if &auth_type == "password" {
        IMAPAuth::Password {
            id: env::var(LOGIN_ID_KEY)?,
            password: env::var(LOGIN_PASSWORD_KEY)?,
        }
    } else if &auth_type == "oauth" {
        IMAPAuth::OAuth {
            user_id: env::var(LOGIN_ID_KEY)?,
            client_id: env::var(IMAP_CLIENT_ID_KEY)?,
            client_secret: env::var(IMAP_CLIENT_SECRET_KEY)?,
            auth_url: env::var(IMAP_AUTH_URL_KEY)?,
            token_url: env::var(IMAP_TOKEN_URL_KEY)?,
            redirect_url: env::var(IMAP_REDIRECT_URL_KEY)?,
        }
    } else {
        panic!("Not supported auth type.");
    };

    let mut receiver = EmailReceiver::construct(&domain_name, port, imap_auth).await?;
    let mut sender: EmailSenderClient = EmailSenderClient::new(
        env::var(LOGIN_ID_KEY)?.as_str(),
        env::var(LOGIN_PASSWORD_KEY)?.as_str(),
        Some(env::var(SMTP_DOMAIN_NAME_KEY)?.as_str()),
    );
    loop {
        receiver.wait_new_email()?;
        println!("new email!");
        let fetches = receiver.retrieve_new_emails()?;
        for fetched in fetches.into_iter() {
            for fetch in fetched.into_iter() {
                if let Some(e) = fetch.envelope() {
                    println!(
                        "from: {}",
                        String::from_utf8(e.from.as_ref().unwrap()[0].name.unwrap().to_vec())
                            .unwrap()
                    );
                    let subject_str = String::from_utf8(e.subject.unwrap().to_vec()).unwrap();
                    println!("subject: {}", subject_str);
                } else {
                    println!("no envelope");
                    break;
                }
                if let Some(b) = fetch.body() {
                    let body = String::from_utf8(b.to_vec())?;
                    println!("body: {}", body);
                    validate_email(&body.as_str(), &sender).await;
                    handle_email(body, &zk_email_circom_path).await;
                } else {
                    println!("no body");
                    break;
                }
            }
        }
        // tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }
}
