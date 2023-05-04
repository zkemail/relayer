use lettre::{
    message::{
        header::{Cc, From, Header, HeaderName, InReplyTo, ReplyTo, To},
        Mailbox, Mailboxes, MessageBuilder,
    },
    transport::smtp::{
        authentication::Credentials, client::SmtpConnection, commands::*, extension::ClientId,
        SMTP_PORT,
    },
    Address, Message, SmtpTransport, Transport,
};
use mailparse::{parse_mail, MailHeaderMap};

use anyhow::{anyhow, Result};
use native_tls::{Protocol, TlsConnector};

#[derive(Clone)]
pub struct SmtpClient {
    email_id: String,
    transport: SmtpTransport,
}

impl SmtpClient {
    pub fn construct(email_id: &str, email_app_password: &str, domain_name: &str) -> Self {
        let smtp_address = domain_name;
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

    pub fn reply_all(&self, raw_email: &str, reply_body: &str) -> Result<()> {
        let pased_email = parse_mail(raw_email.as_bytes())?;

        let original_to = pased_email.headers.get_all_values("To");
        let original_cc = pased_email.headers.get_all_values("Cc");
        let original_from = pased_email
            .headers
            .get_first_value("From")
            .ok_or(anyhow!("No from"))?;
        let in_reply_to = pased_email
            .headers
            .get_first_value("Message-ID")
            .ok_or(anyhow!("No message id"))?;
        let original_subject = pased_email
            .headers
            .get_first_value("Subject")
            .ok_or(anyhow!("No subject"))?;

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

        let mboxes: Mailboxes = From::parse(&original_from)
            .map_err(|e| anyhow!("from parse error: {}", e))?
            .into();
        for mbox in mboxes {
            if mbox.email != sender.email {
                email = email.to(mbox);
            }
        }
        for to in original_to {
            let mboxes: Mailboxes = To::parse(&to)
                .map_err(|e| anyhow!("to parse error: {}", e))?
                .into();
            for mbox in mboxes {
                if mbox.email == self.email_id.parse::<Address>()? {
                    continue;
                }
                email = email.to(mbox);
            }
        }

        for cc in original_cc {
            let mboxes: Mailboxes = Cc::parse(&cc)
                .map_err(|e| anyhow!("cc parse error: {}", e))?
                .into();
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
                return Err(anyhow!("Error building email: {:?}", e));
            }
        };

        println!("Sending email reply-all: {:?}", message);
        self.transport.send(&message)?;
        println!("Sent email reply!");

        Ok(())
    }
}
