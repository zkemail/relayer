// mod chain;
mod config;
mod coordinator;
mod imap_client;
mod parse_email;
mod processer;
mod smtp_client;
mod strings;
use anyhow::{anyhow, Result};
use config::{
    IMAP_AUTH_TYPE_KEY, IMAP_AUTH_URL_KEY, IMAP_CLIENT_ID_KEY, IMAP_CLIENT_SECRET_KEY,
    IMAP_DOMAIN_NAME_KEY, IMAP_PORT_KEY, IMAP_REDIRECT_URL_KEY, IMAP_TOKEN_URL_KEY, LOGIN_ID_KEY,
    LOGIN_PASSWORD_KEY, SMTP_DOMAIN_NAME_KEY, SMTP_PORT_KEY, ZK_EMAIL_PATH_KEY,
};
use coordinator::{handle_email, send_to_modal, validate_email};
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
