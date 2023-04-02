use duct::cmd;
use futures::lock::Mutex;
use serde_json::{json, Value};
use std::error::Error;
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub async fn call_generate_inputs(
    raw_email: &String,
    eth_address: &str,
    nonce: u64,
) -> Result<Value, Box<dyn Error>> {
    let script = format!(
        r#"
        const fs = require('fs');
        const ts = require('typescript');
        const code = fs.readFileSync('~/zk-email-verify/src/scripts/generate_inputs.ts', 'utf-8');
        const compiled = ts.transpile(code);
        const func = new Function('email_base64', 'eth_address', `return requireFromString(compiled, 'generate_inputs.ts').generate_inputs(email, eth_address, nonce);`);
        func('{}', '{}', '{}');
        "#,
        raw_email, eth_address, nonce
    );

    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Failed to run node.js script")
        .stdout;

    let result: Value = serde_json::from_slice(&output)?;

    Ok(result)
}
