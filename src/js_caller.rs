use duct::cmd;
use futures::lock::Mutex;
use serde_json::{json, Value};
use std::error::Error;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

async fn call_generate_inputs(
    email: &str,
    eth_address: &str,
    nonce: usize,
) -> Result<Value, Box<dyn Error>> {
    let script = format!(
        r#"
        const fs = require('fs');
        const ts = require('typescript');
        const code = fs.readFileSync('generate_inputs.ts', 'utf-8');
        const compiled = ts.transpile(code);
        const func = new Function('email', 'eth_address', 'nonce', `return requireFromString('{}', 'generate_inputs.ts').generate_inputs(email, eth_address, nonce);`);
        func('{}', '{}', {});
        "#,
        compiled.replace("`", "\\`"),
        email,
        eth_address,
        nonce
    );

    let output = cmd!("node", "-e", script).read()?;
    let result: Value = serde_json::from_str(&output)?;

    Ok(result)
}
