pub mod js_caller;
pub mod parse_email;
use axum::{
    body::Body,
    extract::{Extension, Json, Path},
    handler::Handler,
    http::StatusCode,
    middleware::AddExtension,
    response::IntoResponse,
    response::Response,
    routing::post,
    Router,
};
use futures::future::BoxFuture;
use futures::lock::Mutex;
use js_caller::call_generate_inputs;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
// use mail_auth::arc::Signature;
use parse_email::*;
// use dkim::Dkim;
use dotenv::dotenv;
// use hyper::Server;
// use mailparse::{parse_header, MailHeaderMap};
use async_trait::async_trait;
use duct::cmd;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::hash_map::DefaultHasher,
    env,
    error::Error,
    hash::{Hash, Hasher},
    {convert::Infallible, net::SocketAddr},
};
// use tokio::sync::Mutex;
// use tower::{service_fn, AddExtensionLayer, ServiceBuilder};
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

async fn process_email_event(payload: Json<Vec<EmailEvent>>) -> impl IntoResponse {
    let re = Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc) to (.+@.+(\..+)+)").unwrap();
    for email in &*payload {
        if let Some(raw_email) = &email.dkim {
            // Hash raw_email
            let hash = {
                let mut hasher = DefaultHasher::new();
                raw_email.hash(&mut hasher);
                hasher.finish()
            };

            let (parsed_headers, body_bytes, key_bytes, signature_bytes) =
                parse_external_eml(raw_email).await.unwrap();
            print!(
                "Parsed email with hash {:?}: {:?} {:?} {:?} {:?}",
                hash, parsed_headers, body_bytes, key_bytes, signature_bytes
            );
            let value = call_generate_inputs(
                raw_email,
                "0x0000000000000000000000000000000000000000",
                hash,
            )
            .await
            .unwrap();
            if let (Some(to), Some(subject)) = (&email.to, &email.subject) {
                let subject_regex = re.clone();
                if subject_regex.is_match(subject) {
                    let custom_reply = format!("{} on Ethereum", subject);
                    let confirmation = send_custom_reply(to, &custom_reply).await;
                    if confirmation {
                        // Call the Rust function that sends a call to Alchemy with the return of that data
                    }
                }
            }
        }
    }

    StatusCode::OK
}

async fn send_custom_reply(to: &str, subject: &str) -> bool {
    let sendgrid_api_key = env::var("SENDGRID_API_KEY").unwrap();
    let client = Client::new();

    let response = client
        .post("https://api.sendgrid.com/v3/mail/send")
        .header("Authorization", format!("Bearer {}", sendgrid_api_key))
        .header("Content-Type", "application/json")
        .body(format!(
            r#"{{
                "personalizations": [{{ "to": [{{ "email": "{}" }}] }}],
                "from": {{ "email": "noreply@example.com" }},
                "subject": "{}",
                "content": [{{ "type": "text/plain", "value": "Are you sure? Remove 'Re:' from the subject when you respond." }}]
            }}"#,
            to, subject
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

#[tokio::main]
async fn main() {
    let nonce = Arc::new(Mutex::new(AtomicUsize::new(1)));
    // Set up a tracing subscriber
    let subscriber = Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set the global tracing subscriber");

    let app = Router::new()
        .route("/webhook", post(process_email_event))
        .route("/emailreceived", post(process_email_event));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
