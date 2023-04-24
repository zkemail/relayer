pub const ZK_EMAIL_PATH_KEY: &'static str = "ZK_EMAIL_CIRCOM_PATH";

pub const IMAP_DOMAIN_NAME_KEY: &'static str = "IMAP_DOMAIN_NAME";
pub const IMAP_PORT_KEY: &'static str = "IMAP_PORT";
pub const IMAP_AUTH_TYPE_KEY: &'static str = "AUTH_TYPE";

pub const IMAP_CLIENT_ID_KEY: &'static str = "IMAP_CLIENT_ID";
pub const IMAP_CLIENT_SECRET_KEY: &'static str = "IMAP_CLIENT_SECRET";
pub const IMAP_AUTH_URL_KEY: &'static str = "IMAP_AUTH_URL";
pub const IMAP_TOKEN_URL_KEY: &'static str = "IMAP_TOKEN_URL";
pub const IMAP_REDIRECT_URL_KEY: &'static str = "IMAP_REDIRECT_URL";

pub const SMTP_DOMAIN_NAME_KEY: &'static str = "SMTP_DOMAIN_NAME";
// pub const SMTP_PORT_KEY: &'static str = "SMTP_PORT";

pub const LOGIN_ID_KEY: &'static str = "LOGIN_ID";
pub const LOGIN_PASSWORD_KEY: &'static str = "LOGIN_PASSWORD";

pub const PROVER_TYPE_KEY: &'static str = "PROVER_TYPE";
pub const EMAIL_DIR_KEY: &'static str = "EMAIL_DIR";
pub const APP_PARAM_PATH_KEY: &'static str = "APP_PARAM_PATH";
pub const AGG_PARAM_PATH_KEY: &'static str = "AGG_PARAM_PATH";
pub const MANIPULATION_DEFS_PATH_KEY: &'static str = "MANIPULATION_DEFS_PATH";

pub const CHAIN_CLIENT_TYPE_KEY: &'static str = "CHAIN_CLIENT_TYPE";
pub const PRIVATE_KEY_HEX_KEY: &'static str = "PRIVATE_KEY_HEX";
pub const RPC_URL_KEY: &'static str = "RPC_URL";
pub const CONTRACT_ADDRESS_KEY: &'static str = "CONTRACT_ADDRESS";
pub const WALLET_ABI_PATH_KEY: &'static str = "WALLET_ABI_PATH";
pub const ERC20_ABI_PATH_KEY: &'static str = "ERC20_ABI_PATH";
pub const CHAIN_ID_KEY: &'static str = "CHAIN_ID";

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RegexType {
    String,
    Uint,
    Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationDef {
    pub(crate) app_config_path: String,
    pub(crate) agg_config_path: String,
    pub(crate) app_pk_path: String,
    pub(crate) agg_pk_path: String,
    pub(crate) max_header_size: usize,
    pub(crate) max_body_size: usize,
    pub(crate) types: Vec<RegexType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationDefsJson {
    pub(crate) rules: HashMap<usize, ManipulationDef>,
}
