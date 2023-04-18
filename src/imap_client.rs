use anyhow::{anyhow, Result};
use imap::types::{Fetch, ZeroCopy};
use imap::{Authenticator, Client, Session};
use native_tls::{self, TlsStream};
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde_json;
use std::io;
use std::net::TcpStream;
use std::slice::Iter;

#[derive(Debug)]
pub struct ImapClient {
    imap_session: Session<TlsStream<TcpStream>>,
}

pub enum IMAPAuth {
    Password {
        id: String,
        password: String,
    },
    OAuth {
        user_id: String,
        client_id: String,
        client_secret: String,
        auth_url: String,
        token_url: String,
        redirect_url: String,
    },
}

pub struct OAuthed {
    user_id: String,
    access_token: String,
}

impl<'a> Authenticator for OAuthed {
    type Response = String;
    #[allow(unused_variables)]
    fn process(&self, data: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user_id, self.access_token
        )
    }
}

impl ImapClient {
    pub async fn construct(domain_name: &str, port: u16, auth: IMAPAuth) -> Result<Self> {
        let tls = native_tls::TlsConnector::builder().build()?;
        let client = imap::connect((domain_name, port), domain_name, &tls)?;
        let mut imap_session = match auth {
            IMAPAuth::Password { id, password } => client.login(id, password).map_err(|e| e.0),
            IMAPAuth::OAuth {
                user_id,
                client_id,
                client_secret,
                auth_url,
                token_url,
                redirect_url,
            } => {
                let oauth_client = BasicClient::new(
                    ClientId::new(client_id),
                    Some(ClientSecret::new(client_secret)),
                    AuthUrl::new(auth_url)?,
                    Some(TokenUrl::new(token_url)?),
                )
                .set_redirect_uri(RedirectUrl::new(redirect_url)?);
                let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
                let (auth_url, csrf_token) = oauth_client
                    .authorize_url(CsrfToken::new_random)
                    // Set the desired scopes.
                    .add_scope(Scope::new("https://mail.google.com/".to_string()))
                    // Set the PKCE code challenge.
                    .set_pkce_challenge(pkce_challenge)
                    .url();
                println!("Browse to: {}", auth_url);
                let mut auth_code = String::new();
                io::stdin().read_line(&mut auth_code)?;
                let token_result = oauth_client
                    .exchange_code(AuthorizationCode::new(auth_code))
                    // Set the PKCE code verifier.
                    .set_pkce_verifier(pkce_verifier)
                    .request_async(async_http_client)
                    .await?;
                let access_token = serde_json::to_string(token_result.access_token())?;
                let oauthed = OAuthed {
                    user_id,
                    access_token,
                };
                client.authenticate("XOAUTH2", &oauthed).map_err(|e| e.0)
            }
        }?;
        imap_session.select("INBOX")?;
        Ok(Self { imap_session })
    }

    pub fn wait_new_email(&mut self) -> Result<()> {
        self.imap_session.idle()?.wait()?;
        Ok(())
    }

    pub fn retrieve_new_emails(&mut self) -> Result<Vec<ZeroCopy<Vec<Fetch>>>> {
        let uids = self.imap_session.uid_search("UNSEEN")?;
        let mut fetches = vec![];
        for (idx, uid) in uids.into_iter().enumerate() {
            if idx >= 10 {
                break;
            }
            println!("uid {}", uid);
            let fetched = self
                .imap_session
                .uid_fetch(uid.to_string(), "(BODY[] ENVELOPE)")?;
            fetches.push(fetched);
        }
        Ok(fetches)
    }
}
