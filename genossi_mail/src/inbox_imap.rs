//! Real `InboxImapClient` implementation backed by `async-imap`.
//!
//! TLS is required. The MVP does not support plaintext IMAP — any mailbox
//! worth using in production offers implicit TLS (port 993). If `imap_tls`
//! is set to `false` in the config, the client logs a warning and still
//! uses TLS. Each operation opens a fresh connection for simplicity and
//! resilience.

use async_imap::Client;
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::{rustls, TlsConnector};

use crate::inbox::{FetchedMessage, ImapConfig, InboxImapClient};
use crate::service::MailServiceError;

type ImapStream = TlsStream<TcpStream>;
type ImapSession = async_imap::Session<ImapStream>;

fn err(msg: impl Into<String>) -> MailServiceError {
    MailServiceError::SmtpError(Arc::from(msg.into()))
}

fn tls_connector() -> TlsConnector {
    let mut root_store = rustls::RootCertStore::empty();
    for anchor in webpki_roots::TLS_SERVER_ROOTS {
        root_store.roots.push(anchor.to_owned());
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

async fn connect_tls(config: &ImapConfig) -> Result<ImapStream, MailServiceError> {
    if !config.tls {
        tracing::warn!("IMAP: imap_tls=false ignored; TLS is required by this MVP");
    }
    let addr = format!("{}:{}", config.host, config.port);
    let tcp = TcpStream::connect(&addr)
        .await
        .map_err(|e| err(format!("IMAP TCP connect to {}: {}", addr, e)))?;
    let connector = tls_connector();
    let server_name = rustls::pki_types::ServerName::try_from(config.host.clone())
        .map_err(|e| err(format!("invalid IMAP host: {}", e)))?;
    connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| err(format!("IMAP TLS handshake: {}", e)))
}

async fn login(config: &ImapConfig) -> Result<Client<ImapStream>, MailServiceError> {
    let stream = connect_tls(config).await?;
    Ok(Client::new(stream))
}

async fn authenticate(
    client: Client<ImapStream>,
    config: &ImapConfig,
) -> Result<ImapSession, MailServiceError> {
    client
        .login(&config.user, &config.pass)
        .await
        .map_err(|(e, _)| err(format!("IMAP login: {}", e)))
}

async fn open_examine_session(
    config: &ImapConfig,
) -> Result<(ImapSession, async_imap::types::Mailbox), MailServiceError> {
    let client = login(config).await?;
    let mut session = authenticate(client, config).await?;
    let mailbox = session
        .examine(&config.mailbox)
        .await
        .map_err(|e| err(format!("IMAP examine {}: {}", config.mailbox, e)))?;
    Ok((session, mailbox))
}

async fn open_select_session(config: &ImapConfig) -> Result<ImapSession, MailServiceError> {
    let client = login(config).await?;
    let mut session = authenticate(client, config).await?;
    session
        .select(&config.mailbox)
        .await
        .map_err(|e| err(format!("IMAP select {}: {}", config.mailbox, e)))?;
    Ok(session)
}

pub struct AsyncImapClient;

impl AsyncImapClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsyncImapClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InboxImapClient for AsyncImapClient {
    async fn uid_validity(&self, config: &ImapConfig) -> Result<i64, MailServiceError> {
        let (mut session, mailbox) = open_examine_session(config).await?;
        let uid_validity = mailbox.uid_validity.unwrap_or(0) as i64;
        let _ = session.logout().await;
        Ok(uid_validity)
    }

    async fn fetch_since(
        &self,
        config: &ImapConfig,
        min_uid: i64,
    ) -> Result<Vec<FetchedMessage>, MailServiceError> {
        let (mut session, _mailbox) = open_examine_session(config).await?;

        let start = (min_uid + 1).max(1);
        let range = format!("{}:*", start);

        let stream = session
            .uid_fetch(range, "(UID BODY.PEEK[])")
            .await
            .map_err(|e| err(format!("IMAP uid_fetch: {}", e)))?;

        let messages: Vec<_> = stream.collect().await;
        let mut out = Vec::new();
        for item in messages {
            let fetch = item.map_err(|e| err(format!("IMAP fetch item: {}", e)))?;
            let uid = match fetch.uid {
                Some(u) => u as i64,
                None => continue,
            };
            if uid <= min_uid {
                continue;
            }
            let raw = fetch.body().map(|b| b.to_vec()).unwrap_or_default();
            out.push(FetchedMessage { uid, raw });
        }

        let _ = session.logout().await;
        Ok(out)
    }

    async fn mark_seen(
        &self,
        config: &ImapConfig,
        uid: i64,
    ) -> Result<(), MailServiceError> {
        let mut session = open_select_session(config).await?;
        let stream = session
            .uid_store(format!("{}", uid), "+FLAGS (\\Seen)")
            .await
            .map_err(|e| err(format!("IMAP uid_store \\Seen: {}", e)))?;
        let _: Vec<_> = stream.collect().await;
        let _ = session.logout().await;
        Ok(())
    }

    async fn move_to_archive(
        &self,
        config: &ImapConfig,
        uid: i64,
    ) -> Result<(), MailServiceError> {
        let archive = config
            .archive_mailbox
            .as_deref()
            .ok_or_else(|| MailServiceError::ConfigMissing(Arc::from("imap_archive_mailbox")))?;

        let mut session = open_select_session(config).await?;
        session
            .uid_mv(format!("{}", uid), archive)
            .await
            .map_err(|e| err(format!("IMAP uid_mv to {}: {}", archive, e)))?;
        let _ = session.logout().await;
        Ok(())
    }

    async fn list_folders(
        &self,
        config: &ImapConfig,
    ) -> Result<Vec<String>, MailServiceError> {
        let stream = connect_tls(config).await?;
        let client = Client::new(stream);
        let mut session = authenticate(client, config).await?;

        let folders_stream = session
            .list(None, Some("*"))
            .await
            .map_err(|e| err(format!("IMAP LIST: {}", e)))?;
        let items: Vec<_> = folders_stream.collect().await;
        let mut names = Vec::new();
        for item in items {
            if let Ok(name) = item {
                names.push(name.name().to_string());
            }
        }
        names.sort();
        let _ = session.logout().await;
        Ok(names)
    }
}
