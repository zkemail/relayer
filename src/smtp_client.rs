use crate::parse_email::{extract_from, extract_recipient_from_subject};
use lettre::{
    message::{
        header::{Cc, From, Header, To},
        Mailbox, Mailboxes,
    },
    transport::smtp::authentication::Credentials,
    Address, Message, SmtpTransport, Transport,
};
use std::error::Error;

#[derive(Clone)]
pub struct EmailSenderClient {
    email_id: String,
    transport: SmtpTransport,
}

impl EmailSenderClient {
    pub fn new(email_id: &str, email_app_password: &str, domain_name: Option<&str>) -> Self {
        let smtp_address = domain_name.unwrap_or("smtp.gmail.com");

        let creds = Credentials::new(email_id.to_owned(), email_app_password.to_owned());

        let client = SmtpTransport::relay(smtp_address)
            .unwrap()
            .credentials(creds)
            .build();
        println!("SMTP client initialized");
        Self {
            email_id: email_id.to_owned(),
            transport: client,
        }
    }

    pub fn send_new_email(
        &self,
        email_subject: &str,
        email_body: &str,
        email_to: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let from_mbox = Mailbox::new(None, self.email_id.parse::<Address>()?);
        let to_mbox = Mailbox::new(None, email_to.parse::<Address>()?);

        let email = Message::builder()
            .from(from_mbox)
            .subject(email_subject)
            .to(to_mbox)
            .body(email_body.to_string())?;

        println!("Sending email: {:?}", email);
        self.transport.send(&email)?;
        println!("Sent email!");

        Ok(true)
    }

    /// This function replies to all recipients of the original email (raw_email).
    /// The subject of the reply email is prefixed with "Re: " followed by the original subject.
    /// The original subject is extracted from the raw_email parameter.
    /// If send_to_recipient, the email recipient mentioned in the subject will be added to the final confirmation

    pub fn reply_all(
        &self,
        raw_email: &str,
        reply_body: &str,
        send_to_recipient: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut original_to = vec![];
        let mut original_cc = vec![];
        let mut original_from = None;
        let mut in_reply_to = String::new();
        let mut original_subject = String::new();

        // TODO: Replace this with Sora's code
        for line in raw_email.lines() {
            if line.starts_with("To:") {
                let parsed: Result<To, _> = To::parse(line);
                if let Ok(header) = parsed {
                    original_to.push(header);
                }
            } else if line.starts_with("Cc:") {
                let parsed: Result<Cc, _> = Cc::parse(line);
                if let Ok(header) = parsed {
                    original_cc.push(header);
                }
            } else if line.starts_with("From:") {
                let parsed: Result<From, _> = From::parse(line);
                if let Ok(header) = parsed {
                    original_from = Some(header);
                }
            } else if line.starts_with("Message-ID:") {
                in_reply_to = line.trim_start_matches("Message-ID:").trim().to_string();
            } else if line.starts_with("Subject:") {
                original_subject = line.trim_start_matches("Subject:").trim().to_string();
            }
        }
        println!(
            "Parsed email headers: {:?} {:?} {:?} {:?} {:?}",
            original_to, original_cc, original_from, in_reply_to, original_subject
        );
        // Create the email sender's Mailbox
        let sender = Mailbox::new(
            Some("Relayer".to_string()),
            self.email_id.parse::<Address>()?,
        );

        let mut email = Message::builder()
            .from(sender.clone())
            .subject(format!("Re: {}", original_subject))
            .in_reply_to(in_reply_to);

        // TODO: The extract_from function is a bit messy, but this is just a backup for error handling in the SMTP client and may not even reply correctly...
        let mboxes: Mailboxes = match original_from {
            Some(from) => from.into(),
            None => {
                let extracted = extract_from(raw_email)?;
                println!("Extracted from as a backup: {:?}", extracted);
                let parsed_mailbox: Result<Mailbox, _> = extracted.parse();
                match parsed_mailbox {
                    Ok(mailbox) => Mailboxes::new().with(mailbox),
                    Err(_) => Mailboxes::new(),
                }
            }
        };

        for mbox in mboxes {
            if mbox.email != sender.email {
                email = email.to(mbox);
            }
        }
        for to in original_to {
            let mboxes: Mailboxes = to.into();
            for mbox in mboxes {
                if mbox.email == self.email_id.parse::<Address>()? {
                    continue;
                }
                email = email.to(mbox);
            }

            if send_to_recipient {
                // Extract and send to any email address from the subject
                print!("Searching for email in subject... {:?}", original_subject);
                match extract_recipient_from_subject(original_subject.as_str()) {
                    Ok(recipient_email) => {
                        let recipient = Mailbox::new(None, recipient_email.parse::<Address>()?);
                        email = email.to(recipient);
                    }
                    Err(e) => {
                        println!("Error extracting recipient from subject: {:?}", e);
                    }
                }
            }
        }

        for cc in original_cc {
            let mboxes: Mailboxes = cc.into();
            for mbox in mboxes {
                if mbox.email == self.email_id.parse::<Address>()? {
                    continue;
                }
                email = email.cc(mbox);
            }
        }

        let message = match email.body(reply_body.as_bytes().to_vec()) {
            Ok(m) => m,
            Err(e) => {
                println!("Error building email: {:?}", e);
                return Err(Box::new(e));
            }
        };

        println!("Sending email reply-all: {:?}", message);
        self.transport.send(&message)?;
        println!("Sent email reply!");

        Ok(())
    }
}
