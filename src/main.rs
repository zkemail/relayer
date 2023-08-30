#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(deprecated)]

pub mod args;
pub mod chain;
pub mod config;
pub mod coordinator;
pub mod db;
pub mod imap_client;
pub mod parse_email;
pub mod processer;
pub mod smtp_client;
pub mod strings;

use crate::parse_email::{extract_from, extract_subject};
use anyhow::{anyhow, Result};
use args::*;
use chain::query_balance;
use clap::Parser;
use config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, ZK_EMAIL_PATH_KEY,
};
use coordinator::{
    calculate_hash, handle_email, validate_email_envelope, BalanceRequest, ValidationStatus,
};
use db::{
    get_pending_and_unvalidated_emails, migrate_email_dbs, set_email_state,
    update_email_state_with_raw_email, EmailData,
};
use dotenv::dotenv;
use imap_client::{IMAPAuth, ImapClient};
use smtp_client::EmailSenderClient;
use std::{collections::VecDeque, env};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = CLI::parse();

    match cli.command {
        Commands::Chain { dir, nonce } => {
            chain::send_to_chain(false, &dir, &nonce).await?;
        }
        Commands::Relayer => run_relayer().await?,
        Commands::Migrate => migrate_email_dbs().await?,
    }

    Ok(())
}

/// This function is the main entry point for the email relayer. It initializes the environment,
/// sets up the email receiver and sender, and enters a loop where it continuously checks for new emails.
/// When a new email is detected, it is processed and added to a queue for further handling.
/// The function is asynchronous and returns a Result.
///
/// # Environment Variables
///
/// * `IMAP_DOMAIN_NAME_KEY` - The domain name for the IMAP server.
/// * `ZK_EMAIL_PATH_KEY` - The path to the zk_email_circom.
/// * `IMAP_PORT_KEY` - The port number for the IMAP server.
/// * `IMAP_AUTH_TYPE_KEY` - The authentication type for the IMAP server.
/// * `LOGIN_ID_KEY` - The login ID for the email account.
/// * `LOGIN_PASSWORD_KEY` - The password for the email account.
/// * `SMTP_DOMAIN_NAME_KEY` - The domain name for the SMTP server.
///
/// # Returns
///
/// * `Result<()>` - The function returns a Result. If the email relayer runs successfully, it returns Ok(()), otherwise it returns an Err.
///
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

    // Re-queue emails that haven't been fully validated or sent yet
    let mut email_queue = VecDeque::new();
    let pending_and_unvalidated_emails = get_pending_and_unvalidated_emails().await?;
    for email_data in pending_and_unvalidated_emails {
        email_queue.push_back(email_data);
    }

    loop {
        // Process emails in the queue in a nonblocking manner
        while let Some(email_data) = email_queue.pop_front() {
            let sender_clone = sender.clone();
            let path_clone = zk_email_circom_path.clone();
            tokio::spawn(async move {
                let result = process_email(&email_data, &sender_clone, &path_clone).await;
                if let Err(e) = result {
                    println!("Error processing email: {}", e);
                }
            });
        }

        // Collect new emails
        println!("Waiting for new email...");
        receiver.wait_new_email().await?;
        println!("New email detected!");
        let fetches = receiver.retrieve_new_emails().await?;
        for fetched in fetches.into_iter() {
            for fetch in fetched.iter() {
                if let Some(b) = fetch.body() {
                    let from_addr: String;
                    let subject_str: String;
                    if let Some(e) = fetch.envelope() {
                        from_addr = {
                            let tag = &e.from.as_ref();
                            println!("from {:?}", tag.ok_or(anyhow!("No from"))?[0]);
                            let former = tag.ok_or(anyhow!("No from"))?[0]
                                .mailbox
                                .clone()
                                .ok_or(anyhow!("No former part of the from address"))?;
                            let latter = tag.ok_or(anyhow!("No from"))?[0]
                                .host
                                .clone()
                                .ok_or(anyhow!("No latter part of the from address"))?;
                            let address = format!(
                                "{}@{}",
                                String::from_utf8(former.to_vec())?,
                                String::from_utf8(latter.to_vec())?
                            );
                            address
                        };
                        println!("from address: {}", from_addr);
                        subject_str =
                            String::from_utf8(e.subject.as_ref().unwrap().to_vec()).unwrap();
                        println!("subject: {}", subject_str);
                    } else {
                        println!("no envelope");
                        break;
                    }
                    let body = String::from_utf8(b.to_vec())?;
                    println!("body: {}", body);

                    // Insert the email into the database with Unvalidated status
                    let hash = calculate_hash(&body);
                    set_email_state(
                        &body,
                        &from_addr,
                        &subject_str,
                        ValidationStatus::Unvalidated,
                    )
                    .await?;

                    // Generate unvalidated EmailData and push it to the validation queue for further processing
                    let email_data = EmailData {
                        body: body.clone(),
                        from: from_addr.clone(),
                        subject: subject_str.clone(),
                        state: ValidationStatus::Unvalidated,
                    };
                    email_queue.push_back(email_data);
                } else {
                    // If there's no body, parse those fields out of the raw header data instead
                    // TODO: Fetch.header is None but the email is printed so it's there somewhere
                    // Print all the fields on fetch
                    // Wrap the code in a try-catch block to handle potential non-existent fields
                    println!("Fetch flags: {:?}", fetch.flags());
                    println!("Fetch body structure: {:?}", fetch.bodystructure());
                    println!("Fetch internal date: {:?}", fetch.internal_date());
                    println!("Fetch UID: {:?}", fetch.uid);
                    println!("Fetch envelope: {:?}", fetch.envelope());

                    let raw_header = std::str::from_utf8(fetch.header().unwrap_or(&[]))?;
                    let from_addr = extract_from(raw_header).unwrap_or("".to_string());
                    let subject_str = extract_subject(raw_header).unwrap_or("".to_string());
                    println!("From address if broken: {}", from_addr);
                    println!("Subject if broken: {}", subject_str);

                    // Insert the email into the database with Unvalidated status
                    let hash = calculate_hash(&raw_header.to_string());
                    set_email_state(
                        raw_header,
                        &from_addr,
                        &subject_str,
                        ValidationStatus::Unvalidated,
                    )
                    .await?;

                    // Generate unvalidated EmailData and push it to the validation queue for further processing
                    let email_data = EmailData {
                        body: raw_header.to_string(),
                        from: from_addr.clone(),
                        subject: subject_str.clone(),
                        state: ValidationStatus::Unvalidated,
                    };
                    email_queue.push_back(email_data);
                    break;
                }
            }
        }
    }
}

/// This function processes an email. It first validates the email envelope and updates the email state to Pending.
/// Depending on the validation status, it either handles the email, queries the balance, or returns an error.
/// The function is asynchronous and returns a Result.
///
/// # Arguments
///
/// * `email_data` - A reference to the EmailData struct containing the email body, from address, subject, and state.
/// * `sender` - A reference to the EmailSenderClient struct.
/// * `zk_email_circom_path` - A string slice that holds the path to the zk_email_circom.
///
/// # Returns
///
/// * `Result<()>` - The function returns a Result. If the email processing is successful, it returns Ok(()), otherwise it returns an Err.
///

async fn process_email(
    email_data: &EmailData,
    sender: &EmailSenderClient,
    zk_email_circom_path: &str,
) -> Result<()> {
    // Validates any unvalidated/pending emails, but don't pending (already-validated email replies)
    let validation = validate_email_envelope(
        email_data.body.as_str(),
        sender,
        email_data.from.as_str(),
        email_data.subject.as_str(),
        Some(email_data.state == ValidationStatus::Unvalidated),
    )
    .await;
    update_email_state_with_raw_email(&email_data.body, ValidationStatus::Pending).await?;

    match validation {
        Ok((validation_status, salt_sender, salt_receiver, balance_request)) => {
            // Calculate the nonce used in the filename
            let file_id = if validation_status == ValidationStatus::Failure
                || validation_status == ValidationStatus::Unvalidated
            {
                String::new()
            } else {
                format!(
                    "({})_({})_({})",
                    salt_sender.unwrap(),
                    salt_receiver.unwrap().as_str(),
                    calculate_hash(&email_data.body)
                )
            };
            println!("File ID/Nonce: {}", file_id);
            println!("Validation status: {:?}", validation_status);

            let email_handle_result = match validation_status {
                ValidationStatus::Ready => {
                    let handled = handle_email(
                        email_data.body.clone(),
                        zk_email_circom_path.clone(),
                        Some(file_id),
                    )
                    .await;
                    set_email_state(
                        &email_data.body,
                        &email_data.from,
                        &email_data.subject,
                        ValidationStatus::Ready,
                    )
                    .await?;
                    handled
                }
                ValidationStatus::Pending => {
                    let BalanceRequest {
                        address,
                        amount,
                        token_name,
                    } = balance_request.unwrap();
                    let zk_email_circom_path = zk_email_circom_path.to_string().clone();
                    set_email_state(
                        &email_data.body,
                        &email_data.from,
                        &email_data.subject,
                        ValidationStatus::Pending,
                    )
                    .await?;
                    let email_data = email_data.clone();
                    let validation_future = tokio::task::spawn(async move {
                        loop {
                            let valid =
                                match query_balance(false, address.as_str(), token_name.as_str())
                                    .await
                                {
                                    Ok(balance) => {
                                        let cloned_amount = amount.clone();
                                        println!("Balance of address {}: {}", address, balance);
                                        let amount_f64 =
                                            cloned_amount.parse::<f64>().unwrap_or(0.0);
                                        balance >= amount_f64
                                    }
                                    Err(error) => {
                                        println!("error: {}", error);
                                        false
                                    }
                                };
                            if valid {
                                break;
                            }
                            // Wait between 4 and 60 seconds to query again, staggering queries to avoid alchemy ratelimits
                            let random_duration = rand::random::<u64>() % 56 + 4;
                            tokio::time::sleep(tokio::time::Duration::from_secs(random_duration))
                                .await;
                        }

                        // TODO: Only set state to READY once the email has been handled and we see a tx on etherscan with the right nullifier
                        match handle_email(
                            email_data.body.clone(),
                            &zk_email_circom_path.clone().to_string(),
                            Some(file_id),
                        )
                        .await
                        {
                            Ok(_) => {
                                match set_email_state(
                                    &email_data.body,
                                    &email_data.from,
                                    &email_data.subject,
                                    ValidationStatus::Ready,
                                )
                                .await
                                {
                                    Ok(_) => println!("Email handled successfully"),
                                    Err(e) => println!("Error setting email state: {}", e),
                                }
                            }
                            Err(e) => println!("Error handling email: {}", e),
                        }
                        // });
                    });
                    Ok(())
                }
                ValidationStatus::Failure => {
                    set_email_state(
                        &email_data.body,
                        &email_data.from,
                        &email_data.subject,
                        ValidationStatus::Failure,
                    )
                    .await?;
                    return Err(anyhow!("Validation failed"));
                }
                ValidationStatus::Unvalidated => {
                    // Note that this scenario should never be reached
                    set_email_state(
                        &email_data.body,
                        &email_data.from,
                        &email_data.subject,
                        ValidationStatus::Unvalidated,
                    )
                    .await?;
                    return Err(anyhow!("Validation unvalidated"));
                }
            };
        }
        Err(error) => {
            // Handle the error case here
            return Err(anyhow!("Error processing email: {}", error));
        }
    }
    Ok(())
}
