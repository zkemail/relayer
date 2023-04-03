pub mod parse_email;
pub mod sh_caller;
use async_trait::async_trait;
use axum::{
    extract::{Extension, Json, Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use dotenv::dotenv;
use duct::cmd;
use futures_util::stream::StreamExt;
use parse_email::*;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use sh_caller::run_commands;
use std::{
    collections::hash_map::DefaultHasher,
    env,
    error::Error,
    fs,
    hash::{Hash, Hasher},
    {convert::Infallible, net::SocketAddr},
};
use tracing_subscriber::{
    fmt::{Subscriber, SubscriberBuilder},
    layer::SubscriberExt,
    prelude::*,
    EnvFilter,
};

#[derive(Debug, Deserialize)]
struct EmailEvent {
    dkim: Option<String>,
    subject: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

// async fn process_email_event(payload: Json<Vec<EmailEvent>>) -> impl IntoResponse {
//     print!("Email received! {:?}", payload);
//     let re = Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc) to (.+@.+(\..+)+)").unwrap();
//     for email in &*payload {
//         if let Some(raw_email) = &email.dkim {
//             // Hash raw_email
//             let hash = {
//                 let mut hasher = DefaultHasher::new();
//                 raw_email.hash(&mut hasher);
//                 hasher.finish()
//             };
//             let email = raw_email.clone() + "\n";
//             let file_path = "./email_before.txt";
//             match fs::write(file_path, email.clone()) {
//                 Ok(_) => println!("Data written successfully to {}", file_path),
//                 Err(e) => println!("Error writing data to file: {}", e),
//             }

//             let (parsed_headers, body_bytes, key_bytes, signature_bytes) =
//                 parse_external_eml(&email).await.unwrap();
//             print!(
//                 "Parsed email with hash {:?}: {:?} {:?} {:?} {:?}",
//                 hash, parsed_headers, body_bytes, key_bytes, signature_bytes
//             );

//             let value = call_generate_inputs(
//                 raw_email,
//                 "0x0000000000000000000000000000000000000000",
//                 hash,
//             )
//             .await
//             .unwrap();
// if let (Some(to), Some(subject)) = (&email.to, &email.subject) {
//     let subject_regex = re.clone();
//     if subject_regex.is_match(subject) {
//         let custom_reply = format!("{} on Ethereum", subject);
//         let confirmation = send_custom_reply(to, &custom_reply).await;
//         if confirmation {
//             // Call the Rust function that sends a call to Alchemy with the return of that data
//         }
//     }
// }
//         }
//     }

//     StatusCode::OK
// }

async fn parse_email_multipart(mut multipart: Multipart) {
    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        if name == String::from("email") {
            // Hash raw_email
            let hash = {
                let mut hasher = DefaultHasher::new();
                data.hash(&mut hasher);
                hasher.finish()
            };
            // let v = data.to_vec();
            let mut transformed_data = Vec::new();
            let mut prev_byte = 0;
            for &byte in data.iter() {
                if byte == 10 {
                    if prev_byte != 13 {
                        transformed_data.push(13);
                    }
                }
                transformed_data.push(byte);
                prev_byte = byte;
            }

            println!("Raw data {:?}", transformed_data.to_vec());
            let raw_email = String::from_utf8(transformed_data).unwrap();

            let (parsed_headers, body_bytes, key_bytes, signature_bytes) =
                parse_external_eml(&raw_email).await.unwrap();

            println!(
                "Parsed email with hash {:?}: {:?} {:?} {:?} {:?}",
                hash,
                String::from_utf8(parsed_headers).unwrap(),
                body_bytes,
                key_bytes,
                signature_bytes,
            );

            let mut subject = extract_subject(&raw_email).unwrap();
            let mut from = extract_from(&raw_email).unwrap();
            println!("Subject, from: {:?} {:?}", subject, from);
            // Write raw_email to ../wallet_{hash}.eml
            let file_path = format!("../wallet_{}.eml", hash);
            match fs::write(file_path.clone(), raw_email.clone()) {
                Ok(_) => println!("Email data written successfully to {}", file_path),
                Err(e) => println!("Error writing data to file: {}", e),
            }

            run_commands(hash);

            // TODO: Swap order and do this first
            // send_custom_reply(&from, &subject).await;
        }
        println!("Content of `{}` is {:?}", name, data);
    }
}

async fn send_custom_reply(to: &str, subject: &str) -> bool {
    let sendgrid_api_key = env::var("SENDGRID_API_KEY").unwrap();
    let client = Client::new();

    println!("Subject: {:?}", subject);
    let subject_regex =
        Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc) to (.+@.+(\..+)+)").unwrap();
    let mut reply_body = "";
    let success = String::from("Sending tx on Ethereum! Executing: ") + subject;
    if subject_regex.is_match(subject) {
        reply_body = success.as_str();
    } else {
        reply_body = "Not formatted correctly! Try 'Send X eth to zkemailverify@gmail.com'";
    }

    let response = client
        .post("https://api.sendgrid.com/v3/mail/send")
        .header("Authorization", format!("Bearer {}", sendgrid_api_key))
        .header("Content-Type", "application/json")
        .body(format!(
            r#"{{
                "personalizations": [{{ "to": [{{ "email": "{}" }}] }}],
                "from": {{ "email": "verify@sendeth.org" }},
                "subject": "{}",
                "content": [{{ "type": "text/plain", "value": "{}" }}]
            }}"#,
            to, subject, reply_body
        ))
        .send()
        .await;
    match response {
        Ok(response) => {
            println!("Response: {:?}", response);
            true
        }
        Err(err) => {
            println!("Error responding: {:?}", err);
            false
        }
    }
}

async fn handle_data(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    body: Json<serde_json::Value>,
) -> impl IntoResponse {
    println!("Handling email {:?}", body);

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json("OK"))
}

#[tokio::main]
async fn main() {
    // Set up a tracing subscriber
    println!("Starting webserver!");
    let subscriber = Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set the global tracing subscriber");

    let app = Router::new()
        .route("/webhook", post(handle_data))
        .route("/email_in", post(parse_email_multipart))
        .route("/email_event", post(handle_data));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    println!("Finished setting up webservers!");
}
