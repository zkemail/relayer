use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "Email Wallet Relayer", version = "0")]
#[command(disable_help_subcommand = true)]
pub struct CLI {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Runs send_to_chain")]
    Chain {
        force_localhost: String,
        dir: String,
        nonce: String,
    },
    #[command(about = "Runs relayer")]
    Relayer,
    #[command(about = "Migrate db")]
    Migrate,
}
