pub mod config;
pub mod coordinator;
pub mod imap_client;
pub mod parse_email;
pub mod processer;
pub mod smtp_client;
pub mod strings;
pub mod chain;
use anyhow::{anyhow, Result};
use ethers_core::types::U256;
use core::future::Future;
use chain::get_token_balance;
use coordinator::{calculate_address, BalanceRequest};
use config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, SMTP_PORT_KEY, ZK_EMAIL_PATH_KEY,
};
use coordinator::{handle_email, send_to_modal, ValidationStatus, validate_email};
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
    pin::Pin,
    boxed::Box
};
use strings::{first_reply, invalid_reply};

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

    let mut receiver = EmailReceiver::construct(&domain_name, port, imap_auth.clone()).await?;
    let mut sender: EmailSenderClient = EmailSenderClient::new(
        env::var(LOGIN_ID_KEY)?.as_str(),
        env::var(LOGIN_PASSWORD_KEY)?.as_str(),
        Some(env::var(SMTP_DOMAIN_NAME_KEY)?.as_str()),
    );
    println!("Email receiver constructed with auto-reconnect.");
    loop {
        receiver
            .wait_new_email(&domain_name, port, &imap_auth.clone())
            .await?;
        println!("new email!");
        let fetches = receiver
            .retrieve_new_emails(&domain_name, port, &imap_auth.clone())
            .await?;
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
                    let validation: Result<(ValidationStatus, Option<String>, Option<String>, Option<BalanceRequest>)> = validate_email(&body.as_str(), &sender).await;
                    match validation {
                        Ok((validation_status, salt_sender, salt_receiver, balance_request)) => {
                            let file_id = salt_sender.unwrap() + "_" + salt_receiver.unwrap().as_str();
                            let email_handle_result = match validation_status {
                                ValidationStatus::Ready =>
                                    handle_email(body, &zk_email_circom_path, Some(file_id)).await,
                                ValidationStatus::Pending => {
                                    let BalanceRequest {
                                        address,
                                        amount,
                                        token_name,
                                    } = balance_request.unwrap();
                                    let validation_future = tokio::task::spawn(async move {
                                        loop {
                                            let valid = match get_token_balance(false, address.as_str(), token_name.as_str()).await {
                                                Ok(balance) => {
                                                    let cloned_amount = amount.clone();
                                                    println!("balance: {}", balance);
                                                    let amount_u256 = U256::from_dec_str(&cloned_amount).unwrap_or_else(|_| U256::zero());
                                                    balance >= amount_u256
                                                },
                                                Err(error) => {
                                                    println!("error: {}", error);
                                                    false
                                                }
                                            };
                                            if valid {
                                                break;
                                            }
                                            tokio::time::sleep( tokio::time::Duration::from_millis(1000)).await;
                                        }
                                    });
                                    match validation_future.await {
                                        Ok(_) => {
                                            handle_email(body, &zk_email_circom_path, Some(file_id)).await
                                        }
                                        Err(e) => {
                                            println!("Pending validation error: {}", e);
                                            Err(anyhow!("Pending validation failed"))
                                        }
                                    }
                                },
                                ValidationStatus::Failure => {
                                    return Err(anyhow!("Validation failed"));
                                }  
                            };
                        }
                        Err(error) => {
                            // Handle the error case here   
                        }
                    }
                    

                } else {
                    println!("no body");
                    break;
                }
            }
        }
        // tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }
}
