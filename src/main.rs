use anyhow::{anyhow, Result};
use dotenv::dotenv;
use relayer::chain_client::Halo2Client;
use relayer::config::*;
use relayer::imap_client::{IMAPAuth, ImapClient};
// use parse_email::*;
// use parse_email::*;
use relayer::processer::EmailProcesser;
use relayer::prover::Halo2SimpleProver;
use relayer::smtp_client::SmtpClient;
use std::env;

// #[derive(Debug, Deserialize)]
// struct EmailEvent {
//     dkim: Option<String>,
//     subject: Option<String>,
//     from: Option<String>,
//     to: Option<String>,
// }

// async fn handle_email(raw_email: String, zk_email_circom_dir: &String) {
//     let hash = {
//         let mut hasher = DefaultHasher::new();
//         raw_email.hash(&mut hasher);
//         hasher.finish()
//     };
//     let mut subject = extract_subject(&raw_email).unwrap();
//     let mut from = extract_from(&raw_email).unwrap();
//     println!("Subject, from: {:?} {:?}", subject, from);

//     // Validate subject, and send rejection/reformatting email if necessary
//     // let re = Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc) to (.+@.+(\..+)+)").unwrap();
//     // if let (Some(to), Some(subject)) = (&email.to, &email.subject) {
//     //     let subject_regex = re.clone();
//     //     if subject_regex.is_match(subject) {
//     //         let custom_reply = format!("{} on Ethereum", subject);
//     //         let confirmation = send_custom_reply(to, &custom_reply).await;
//     //     }
//     // }

//     // Path 1: Write raw_email to ../wallet_{hash}.eml
//     let file_path = format!("{}/wallet_{}.eml", "./received_eml", hash);
//     match fs::write(file_path.clone(), raw_email.clone()) {
//         Ok(_) => println!("Email data written successfully to {}", file_path),
//         Err(e) => println!("Error writing data to file: {}", e),
//     }
//     std::thread::sleep(std::time::Duration::from_secs(3));

//     // Path 2: Send to modal
//     // let webhook_url = "";
//     // let client = reqwest::Client::new();
//     // let response = client
//     //     .post(webhook_url)
//     //     .header("Content-Type", "application/octet-stream")
//     //     .body(raw_email)
//     //     .send()
//     //     .await
//     //     .unwrap();

//     // Path

//     // println!("Response status: {}", response.status());
// }

// pub async fn validate_email(raw_email: &str, emailer: &EmailSenderClient) {
//     let mut subject = extract_subject(&raw_email).unwrap();
//     let mut from = extract_from(&raw_email).unwrap();
//     println!("Subject, from: {:?} {:?}", subject, from);

//     // Validate subject, and send rejection/reformatting email if necessary
//     let re = Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc) to (.+@.+(\..+)+)").unwrap();
//     let subject_regex = re.clone();
//     let mut custom_reply: String = "".to_string();
//     if subject_regex.is_match(subject.as_str()) {
//         if let Some(captures) = re.captures(subject.as_str()) {
//             // Extract the amount and recipient from the captures
//             let amount = captures.get(1).map_or("", |m| m.as_str());
//             let recipient = captures.get(4).map_or("", |m| m.as_str());
//             custom_reply = format!("Valid send initiated. Sending {} eth to {} on Ethereum. We will follow up with Etherscan link when finished!", amount, recipient);
//         } else {
//             custom_reply = "Send seems to match regex but is invalid! Please try again with this subject: \"Send _ eth to __@__.___\"".to_string();
//         }
//         println!("Send valid! Validating proof...");
//         // .await;
//     } else {
//         println!("Send invalid! Regex failed...");
//         custom_reply =
//             "Send invalid! Please try again with this subject: \"Send _ eth to __@__.___\""
//                 .to_string();
//     }
//     let confirmation = emailer.reply_all(raw_email, &custom_reply);
// }

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let domain_name = env::var(IMAP_DOMAIN_NAME_KEY)?;
    // let zk_email_circom_path = env::var(ZK_EMAIL_PATH_KEY)?;
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
    // loop {
    //     receiver.wait_new_email()?;
    //     println!("new email!");
    //     let fetches = receiver.retrieve_new_emails()?;
    //     for fetched in fetches.into_iter() {
    //         for fetch in fetched.into_iter() {
    //             if let Some(e) = fetch.envelope() {
    //                 println!(
    //                     "from: {}",
    //                     String::from_utf8(e.from.as_ref().unwrap()[0].name.unwrap().to_vec())
    //                         .unwrap()
    //                 );
    //                 // println!(
    //                 //     "to: {}",
    //                 //     String::from_utf8(e.to.as_ref().unwrap()[0].name.unwrap().to_vec())
    //                 //         .unwrap()
    //                 // );
    //                 let subject_str = String::from_utf8(e.subject.unwrap().to_vec()).unwrap();
    //                 println!("subject: {}", subject_str);
    //             } else {
    //                 println!("no envelope");
    //                 break;
    //             }
    //             if let Some(b) = fetch.body() {
    //                 let body = String::from_utf8(b.to_vec())?;
    //                 println!("body: {}", body);
    //             } else {
    //                 println!("no body");
    //                 break;
    //             }
    //         }
    //     }
    //     // tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    // }
}
