use anyhow::{anyhow, Result};
use relayer::run_relayer;

#[cfg(feature = "ether")]
#[tokio::main]
async fn main() -> Result<()> {
    run_relayer().await
}

#[cfg(not(feature = "ether"))]
fn main() {
    panic!("ether feature must be enabled!");
}
