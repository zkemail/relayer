pub mod parse_email;
use axum::{
    extract::{Extension, Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
// use mail_auth::arc::Signature;
use parse_email::*;
// use dkim::Dkim;
use dotenv::dotenv;
// use hyper::Server;
// use mailparse::{parse_header, MailHeaderMap};
use duct::cmd;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::{
    env,
    error::Error,
    {convert::Infallible, net::SocketAddr},
};
// use tower::{service_fn, ServiceBuilder};
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

fn call_gen_input(eml: &str) -> Result<String, Box<dyn Error>> {
    let script = format!(
        r#"
        const fs = require('fs');
        const ts = require('typescript');
        const code = fs.readFileSync('gen_input.ts', 'utf-8');
        const compiled = ts.transpile(code);
        const func = new Function('eml', `${{compiled}}; return gen_input(eml);`);
        func('{}');
        "#,
        eml
    );

    let output = cmd!("node", "-e", script).read()?;
    let result: Value = serde_json::from_str(&output)?;

    Ok(result.as_str().unwrap().to_string())
}

async fn process_email_event(payload: Json<Vec<EmailEvent>>) -> impl IntoResponse {
    let re = Regex::new(r"[Ss]end ?\$?(\d+(\.\d{1,2})?) (eth|usdc) to (.+@.+(\..+)+)").unwrap();
    for email in &*payload {
        if let Some(raw_email) = &email.dkim {
            let (parsed_headers, body_bytes, key_bytes, signature_bytes) =
                parse_external_eml(raw_email).await.unwrap();
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
    true
}

#[tokio::main]
async fn main() {
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
