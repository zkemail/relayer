pub mod chain;
pub mod config;
pub mod coordinator;
pub mod imap_client;
pub mod parse_email;
pub mod processer;
pub mod smtp_client;
pub mod strings;
use anyhow::{anyhow, Result};
use chain::query_balance;
use config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, SMTP_PORT_KEY, ZK_EMAIL_PATH_KEY,
};
use coordinator::{calculate_address, BalanceRequest};
use coordinator::{handle_email, send_to_modal, validate_email_envelope, ValidationStatus};
use core::future::Future;
use dotenv::dotenv;
use ethers_core::types::U256;
use http::StatusCode;
use imap_client::{IMAPAuth, ImapClient};
use smtp_client::EmailSenderClient;
use std::env;
use strings::{first_reply, invalid_reply};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        Some(arg) => match arg.as_str() {
            "chain" => {
                if args.len() < 5 {
                    println!("Function1 requires three additional parameters: a bool to force localhost [usually false], a directory string, and a nonce string.");
                } else {
                    let force_localhost = args[2]
                        .parse::<bool>()
                        .expect("Error parsing force_localhost. Should be 'true' or 'false'");

                    let dir = &args[3];
                    let nonce = &args[4];

                    chain::send_to_chain(force_localhost, dir, nonce).await?;
                };
                Ok(())
            }
            "relayer" => {
                run_relayer().await?;
                Ok(())
            }
            _ => Err(anyhow!("Invalid function! Use either 'chain' or 'relayer'")),
        },
        None => Err(anyhow!(
            "Please provide a function to call! Use either 'chain' or 'relayer'"
        )),
    }
}

async fn run_relayer() -> Result<()> {
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

    let mut receiver = ImapClient::construct(&domain_name, port, imap_auth.clone()).await?;
    let sender: EmailSenderClient = EmailSenderClient::new(
        env::var(LOGIN_ID_KEY)?.as_str(),
        env::var(LOGIN_PASSWORD_KEY)?.as_str(),
        Some(env::var(SMTP_DOMAIN_NAME_KEY)?.as_str()),
    );
    println!("Email receiver constructed with auto-reconnect.");
    loop {
        println!("Waiting for new email...");
        receiver.wait_new_email().await?;
        println!("New email detected!");
        let fetches = receiver.retrieve_new_emails().await?;
        for fetched in fetches.into_iter() {
            for fetch in fetched.into_iter() {
                if let Some(b) = fetch.body() {
                    let from_addr: String;
                    let subject_str: String;
                    if let Some(e) = fetch.envelope() {
                        from_addr = {
                            let tag = &e.from.as_ref();
                            println!("from {:?}", tag.ok_or(anyhow!("No from"))?[0]);
                            let former = tag.ok_or(anyhow!("No from"))?[0]
                                .mailbox
                                .ok_or(anyhow!("No former part of the from address"))?;
                            let latter = tag.ok_or(anyhow!("No from"))?[0]
                                .host
                                .ok_or(anyhow!("No latter part of the from address"))?;
                            let address = format!(
                                "{}@{}",
                                String::from_utf8(former.to_vec())?,
                                String::from_utf8(latter.to_vec())?
                            );
                            address
                        };
                        println!("from address: {}", from_addr);
                        subject_str = String::from_utf8(e.subject.unwrap().to_vec()).unwrap();
                        println!("subject: {}", subject_str);
                    } else {
                        println!("no envelope");
                        break;
                    }
                    let body = String::from_utf8(b.to_vec())?;
                    println!("body: {}", body);
                    let validation = validate_email_envelope(&body.as_str(), &sender, &from_addr.as_str(), &subject_str.as_str()).await;
                    match validation {
                        Ok((validation_status, salt_sender, salt_receiver, balance_request)) => {
                            // Calculate the nonce used in the filename
                            let file_id = format!(
                                "({})_({})",
                                salt_sender.unwrap(),
                                salt_receiver.unwrap().as_str()
                            );
                            println!("File ID/Nonce: {}", file_id);
                            println!("Validation status: {:?}", validation_status);
                            let email_handle_result = match validation_status {
                                ValidationStatus::Ready => {
                                    handle_email(body, &zk_email_circom_path, Some(file_id)).await
                                }
                                ValidationStatus::Pending => {
                                    let BalanceRequest {
                                        address,
                                        amount,
                                        token_name,
                                    } = balance_request.unwrap();
                                    let validation_future = tokio::task::spawn(async move {
                                        loop {
                                            let valid = match query_balance(
                                                false,
                                                address.as_str(),
                                                token_name.as_str(),
                                            )
                                            .await
                                            {
                                                Ok(balance) => {
                                                    let cloned_amount = amount.clone();
                                                    println!("Balance: {}", balance);
                                                    let amount_u256 =
                                                        U256::from_dec_str(&cloned_amount)
                                                            .unwrap_or_else(|_| U256::zero());
                                                    balance >= amount_u256
                                                }
                                                Err(error) => {
                                                    println!("error: {}", error);
                                                    false
                                                }
                                            };
                                            if valid {
                                                break;
                                            }
                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                4000,
                                            ))
                                            .await;
                                        }
                                    });
                                    match validation_future.await {
                                        Ok(_) => {
                                            handle_email(body, &zk_email_circom_path, Some(file_id))
                                                .await
                                        }
                                        Err(e) => {
                                            println!("Pending validation error: {}", e);
                                            Err(anyhow!("Pending validation failed"))
                                        }
                                    }
                                }
                                ValidationStatus::Failure => {
                                    println!("Validation failed");
                                    continue;
                                }
                            };
                        }
                        Err(error) => {
                            // Handle the error case here
                            println!("Error processing email: {}", error);
                            continue;
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
