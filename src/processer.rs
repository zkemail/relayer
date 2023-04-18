use crate::imap_client::ImapClient;
use crate::prover::EmailProver;
use anyhow::{anyhow, Result};
use cfdkim::{canonicalize_signed_email, SignerBuilder};
use fancy_regex::Regex;
use imap::types::Fetch;

#[derive(Debug)]
pub struct EmailProcesser {
    imap_client: ImapClient,
    // prover: P,
    // num_unprocessed_email: usize,
}

impl EmailProcesser {
    const SUBJECT_REGEX: &'static str = r"(?<=Email Wallet Manipulation ID )([0-9]|\.)+";
    pub fn new(imap_client: ImapClient) -> Self {
        Self {
            imap_client,
            // prover,
            // num_unprocessed_email,
        }
    }

    pub fn fetch_new_emails(&mut self) -> Result<()> {
        let fetches = self.imap_client.retrieve_new_emails()?;
        // println!("The number of fetched emails: {}", fetches.len());
        for fetched in fetches.into_iter() {
            for fetch in fetched.into_iter() {
                match self.fetch_one_email(fetch) {
                    Ok(_) => {
                        // self.num_unprocessed_email += 1;
                        continue;
                    }
                    Err(e) => {
                        println!("{}", e);
                        continue;
                    }
                }
            }
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
        Ok(())
    }

    fn fetch_one_email(&mut self, fetch: &Fetch) -> Result<()> {
        let envelope = fetch.envelope().ok_or(anyhow!("No envelope"))?;
        let subject = envelope.subject.ok_or(anyhow!("No subject"))?;
        println!("subject {:?}", subject);
        let subject_str = {
            let tag = envelope.subject.ok_or(anyhow!("No subject"))?;
            String::from_utf8(tag.to_vec())?
        };
        println!("subject_str {}", subject_str);
        let subject_regex = Regex::new(Self::SUBJECT_REGEX)?;
        let manipulation_id = subject_regex
            .find(&subject_str)?
            .ok_or(anyhow!("No manipulation id"))?
            .as_str()
            .parse::<usize>()?;
        println!("manipulation_id {}", manipulation_id);
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
        let email_bytes = fetch.body().ok_or(anyhow!("No body"))?;
        // self.prover.push_email(email_bytes)?;
        Ok(())
    }

    pub fn wait_new_email(&mut self) -> Result<()> {
        self.imap_client.wait_new_email()
    }
}
