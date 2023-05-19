use anyhow::{anyhow, Result};
use dotenv::dotenv;
#[cfg(feature = "ether")]
use relayer::chain_client::Halo2Client;
use relayer::config::*;
#[cfg(feature = "ether")]
use relayer::imap_client::{IMAPAuth, ImapClient};
// use parse_email::*;
// use parse_email::*;
#[cfg(feature = "ether")]
use relayer::processer::EmailProcesser;
#[cfg(feature = "ether")]
use relayer::prover::Halo2SimpleProver;
#[cfg(feature = "ether")]
use relayer::smtp_client::SmtpClient;
use std::env;

#[cfg(feature = "ether")]
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let domain_name = env::var(IMAP_DOMAIN_NAME_KEY)?;
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

    let imap_client = ImapClient::construct(&domain_name, port, imap_auth).await?;
    let smtp_client = SmtpClient::construct(
        env::var(LOGIN_ID_KEY)?.as_str(),
        env::var(LOGIN_PASSWORD_KEY)?.as_str(),
        env::var(SMTP_DOMAIN_NAME_KEY)?.as_str(),
    );
    let prover = match env::var(PROVER_TYPE_KEY)?.as_str() {
        "halo2-simple" => Halo2SimpleProver::construct(
            env::var(EMAIL_DIR_KEY)?.as_str(),
            env::var(APP_PARAM_PATH_KEY)?.as_str(),
            env::var(AGG_PARAM_PATH_KEY)?.as_str(),
            env::var(MANIPULATION_DEFS_PATH_KEY)?.as_str(),
        )?,
        _ => panic!("Not supported prover type"),
    };
    let chain_client = match env::var(CHAIN_CLIENT_TYPE_KEY)?.as_str() {
        "halo2-client" => Halo2Client::construct(
            env::var(PRIVATE_KEY_HEX_KEY)?.as_str(),
            env::var(RPC_URL_KEY)?.as_str(),
            env::var(CONTRACT_ADDRESS_KEY)?.as_str(),
            env::var(WALLET_ABI_PATH_KEY)?.as_str(),
            env::var(ERC20_ABI_PATH_KEY)?.as_str(),
            env::var(IMAN_ABI_PATH_KEY)?.as_str(),
            env::var(CHAIN_ID_KEY)?.as_str().parse::<u64>()?,
        )?,
        _ => panic!("Not supported chain client type"),
    };

    let mut processer = EmailProcesser::new(
        imap_client,
        smtp_client,
        prover,
        chain_client,
        env::var(SCAN_URL_PREFIX_KEY)?.as_str(),
    );
    loop {
        println!("waiting new emails...");
        processer.wait_new_email()?;
        println!("new emails are found!");
        processer.fetch_new_emails().await?;
        println!("emails are processed.");
        // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
    }
}
#[cfg(not(feature = "ether"))]
fn main() {
    panic!("ether feature must be enabled!");
}
