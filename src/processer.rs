use crate::config::ManipulationDef;
use crate::coordinator::{handle_email, validate_email};
use crate::prover::EmailProver;
use crate::smtp_client::SmtpClient;
use crate::{chain_client::ChainClient, imap_client::ImapClient};
use anyhow::{anyhow, Result};
use cfdkim::{canonicalize_signed_email, SignerBuilder};
use fancy_regex::Regex;
use imap::types::Fetch;

pub struct EmailProcesser<P: EmailProver, C: ChainClient<P>> {
    imap_client: ImapClient,
    smtp_client: SmtpClient,
    prover: P,
    chain_client: C,
    scan_url_prefix: String,
}

impl<P: EmailProver, C: ChainClient<P>> EmailProcesser<P, C> {
    const MANIPULATION_SUBJECT_REGEX: &'static str = r"(?<=Email Wallet Manipulation ID )[0-9]+";
    const QUERY_SUBJECT_REGEX: &'static str = r"(?<=Email Wallet Query My Balance of )[A-Z]+";
    pub fn new(
        imap_client: ImapClient,
        smtp_client: SmtpClient,
        prover: P,
        chain_client: C,
        scan_url_prefix: &str,
    ) -> Self {
        Self {
            imap_client,
            smtp_client,
            prover,
            chain_client, // num_unprocessed_email,
            scan_url_prefix: scan_url_prefix.to_string(),
        }
    }

    pub async fn fetch_new_emails(&mut self) -> Result<()> {
        let fetches = self.imap_client.retrieve_new_emails()?;
        // println!("The number of fetched emails: {}", fetches.len());

        // If circom is set as the PROVER in the env
        let prover = std::env::var("PROVER").unwrap_or("".to_string());
        if prover == "circom" {
            for fetched in fetches.into_iter() {
                for fetch in fetched.into_iter() {
                    if let Some(b) = fetch.body() {
                        let body = String::from_utf8(b.to_vec())?;
                        // let body = self.imap_client.fetch_one_email(fetch).await?;
                        validate_email(&body.as_str(), None).await;
                        handle_email(body).await;
                    }
                }
            }
        } else if prover == "halo2" {
            for fetched in fetches.into_iter() {
                for fetch in fetched.into_iter() {
                    match self.fetch_one_email(fetch).await {
                        Ok(_) => {
                            continue;
                        }
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    }
                }
            }
            self.prover.prove_emails().await?;
            while let Some((id, raw_email, calldata)) = self.prover.pop_calldata().await? {
                let tx_hash = self.chain_client.send_chain(id, calldata).await?;
                // let etherscan_url = format!("https://goerli.etherscan.io/tx/0x{:x}", tx_hash);
                let etherscan_url = format!("{}{:x}", self.scan_url_prefix, tx_hash);
                let reply = format!(
                    "Transaction sent! View Etherscan confirmation: {}.",
                    etherscan_url
                );
                println!("Replying with confirmation...{}", reply);
                self.smtp_client.reply_all(&raw_email, &reply)?;
            }
            // let num_proofs_per_aggregation = self.prover.num_proofs_per_aggregation();
            // let mut num_agg_proof = 0;
            // while self.num_unprocessed_email >= num_proofs_per_aggregation {
            //     self.prover.prove_emails()?;
            //     num_agg_proof += 1;
            //     self.num_unprocessed_email -= num_proofs_per_aggregation;
            // }
            // for _ in 0..num_agg_proof {
            //     let agg_proof = self.prover.pop_proof()?;
            // }
        } else {
            return Err(anyhow!("Invalid prover"));
        }
        Ok(())
    }

    async fn fetch_one_email(&mut self, fetch: &Fetch) -> Result<()> {
        let email_bytes = fetch.body().ok_or(anyhow!("No body"))?;
        let envelope = fetch.envelope().ok_or(anyhow!("No envelope"))?;
        let from_addr = {
            let tag = envelope.from.as_ref();
            println!("from {:?}", tag.ok_or(anyhow!("No from"))?[0]);
            let former = tag.ok_or(anyhow!("No from"))?[0]
                .mailbox
                .ok_or(anyhow!("No former part of the from address"))?;
            let latter = tag.ok_or(anyhow!("No from"))?[0]
                .host
                .ok_or(anyhow!("No latter part of the from address"))?;
            let address = format!(
                "{}@{}",
                String::from_utf8(former.to_vec())?,
                String::from_utf8(latter.to_vec())?
            );
            address
        };
        println!("from adress {}", from_addr);

        let to_addr = {
            let tag = envelope.to.as_ref();
            println!("to {:?}", tag.ok_or(anyhow!("No to"))?[0]);
            let former = tag.ok_or(anyhow!("No to"))?[0]
                .mailbox
                .ok_or(anyhow!("No former part of the to address"))?;
            let latter = tag.ok_or(anyhow!("No to"))?[0]
                .host
                .ok_or(anyhow!("No latter part of the to address"))?;
            let address = format!(
                "{}@{}",
                String::from_utf8(former.to_vec())?,
                String::from_utf8(latter.to_vec())?
            );
            address
        };
        println!("to adress {}", to_addr);

        let subject = envelope.subject.ok_or(anyhow!("No subject"))?;
        println!("subject {:?}", subject);
        let subject_str = {
            let tag = envelope.subject.ok_or(anyhow!("No subject"))?;
            String::from_utf8(tag.to_vec())?
        };
        println!("subject_str {}", subject_str);
        if let Some(subject_match) =
            Regex::new(Self::MANIPULATION_SUBJECT_REGEX)?.find(&subject_str)?
        {
            let manipulation_id = subject_match.as_str().parse::<usize>()?;
            println!("manipulation_id {}", manipulation_id);
            let (header, body, _) = canonicalize_signed_email(email_bytes)?;
            let defs = self.prover.manipulation_defs();
            let def = &defs.rules[&manipulation_id];
            if header.len() > def.max_header_size {
                return Err(anyhow!(
                    "The max header size is {}, but the received header size is {}",
                    def.max_header_size,
                    header.len()
                ));
            } else if body.len() > def.max_body_size {
                return Err(anyhow!(
                    "The max body size is {}, but the received body size is {}",
                    def.max_body_size,
                    body.len()
                ));
            }
            self.prover.push_email(manipulation_id, email_bytes).await?;
        } else if let Some(subject_match) =
            Regex::new(Self::QUERY_SUBJECT_REGEX)?.find(&subject_str)?
        {
            let token_name = subject_match.as_str();
            println!("query token name {}", token_name);
            let balance = self
                .chain_client
                .query_balance(&from_addr, token_name)
                .await?;
            let reply = format!("You have {} {} now.", balance, token_name);
            println!("Replying with confirmation...{}", reply);
            self.smtp_client
                .reply_all(&String::from_utf8(email_bytes.to_vec())?, &reply)?;
        }
        Ok(())
    }

    pub fn wait_new_email(&mut self) -> Result<()> {
        self.imap_client.wait_new_email()
    }
}
