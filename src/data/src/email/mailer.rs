use crate::email::mailer_callback::EMailCallback;
use crate::email::mailer_microsoft_graph::sender_microsoft_graph;
use crate::email::smtp_oauth_token::SmtpOauthToken;
use crate::events::event::Event;
use crate::rauthy_config::{EmailTlsMode, RauthyConfig};
use crate::settings::{self, EffectiveSmtp};
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Mechanism;
use lettre::transport::smtp::{authentication, client};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Deserialize;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmailType {
    Custom,
    EmailChangeConfirm,
    EmailChangeInfo,
    EmailRegisteredAlready,
    LoginLocation,
    Notification,
    PasswordReset,
    PasswordResetInfo,
    OtpRequest,
}

impl Display for EmailType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom => write!(f, "Custom"),
            Self::EmailChangeConfirm => write!(f, "EmailChangeConfirm"),
            Self::EmailChangeInfo => write!(f, "EmailChangeInfo"),
            Self::EmailRegisteredAlready => write!(f, "EmailRegisteredAlready"),
            Self::LoginLocation => write!(f, "LoginLocation"),
            Self::Notification => write!(f, "Notification"),
            Self::PasswordReset => write!(f, "PasswordReset"),
            Self::PasswordResetInfo => write!(f, "PasswordResetInfo"),
            Self::OtpRequest => write!(f, "OtpRequest"),
        }
    }
}

#[derive(Debug)]
pub struct EMail {
    pub typ: EmailType,
    pub recipient_name: String,
    pub address: String,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub enum SmtpConnMode {
    Default,
    XOauth2,
    MicrosoftGraph,
    Test,
}

impl From<&str> for SmtpConnMode {
    fn from(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            "xoauth2" => Self::XOauth2,
            "microsoft_graph" => Self::MicrosoftGraph,
            "test" => Self::Test,
            _ => panic!("Invalid smtp_conn_mode, expected one of: default xoauth2 microsoft_graph"),
        }
    }
}

pub async fn sender(rx: mpsc::Receiver<(EMail, EMailCallback)>) {
    debug!("E-Mail sender started");

    if RauthyConfig::get().vars.email.smtp_conn_mode == SmtpConnMode::MicrosoftGraph {
        sender_microsoft_graph(rx).await
    } else {
        sender_smtp(rx).await
    }
}

/// Builds the message to send. Fails for an invalid sender or recipient instead of panicking.
fn build_message(from: &Mailbox, mut req: EMail) -> Result<lettre::Message, String> {
    // The templates and translations say "Rauthy"; show the configured name instead.
    let app_name = settings::app_name();
    if app_name != "Rauthy" {
        let brand = |s: &str| s.replace("Rauthy", &app_name);
        let html_name = app_name.replace('&', "&amp;");
        req.subject = brand(&req.subject);
        req.text = req.text.as_deref().map(brand);
        req.html = req.html.as_deref().map(|s| s.replace("Rauthy", &html_name));
    }

    let to: Mailbox = format!("{} <{}>", req.recipient_name, req.address)
        .parse()
        .map_err(|err| format!("invalid recipient '{}': {err}", req.address))?;
    let builder = lettre::Message::builder()
        .from(from.clone())
        .to(to)
        .subject(req.subject);

    let msg = match (req.text, req.html) {
        (Some(text), Some(html)) => {
            builder.multipart(MultiPart::alternative_plain_html(text, html))
        }
        (None, Some(html)) => builder.singlepart(SinglePart::html(html)),
        (Some(text), None) => builder.singlepart(SinglePart::plain(text)),
        (None, None) => {
            warn!("Sending E-Mail with empty body!");
            builder.singlepart(SinglePart::plain(String::default()))
        }
    };
    msg.map_err(|err| err.to_string())
}

/// The built-in SMTP sender.
///
/// It never needs a working SMTP server to start: with nothing configured, or a server that cannot
/// be reached, E-Mails are dropped with a log line (and an event for failed sends) until the
/// settings are fixed. The settings are read for every E-Mail, so a change in the admin UI takes
/// effect without a restart.
async fn sender_smtp(mut rx: mpsc::Receiver<(EMail, EMailCallback)>) {
    let vars = &RauthyConfig::get().vars.email;
    let mut transport: Option<(u64, AsyncSmtpTransport<Tokio1Executor>)> = None;
    let mut warned_generation = None;

    loop {
        debug!("Listening for incoming send E-Mail requests");
        let Some((req, callback)) = rx.recv().await else {
            warn!("Received 'None' in email 'sender' - exiting");
            break;
        };
        debug!("New E-Mail for address: {:?}", req.address);

        let cfg = settings::smtp();
        if vars.smtp_conn_mode == SmtpConnMode::Test || !cfg.is_configured() {
            if warned_generation != Some(cfg.generation) {
                warn!(
                    "No SMTP server is configured, E-Mails are NOT sent. Set it up in the admin \
                    UI under Config -> Email."
                );
                warned_generation = Some(cfg.generation);
            }
            continue;
        }

        // (re)connect on the first mail and whenever the settings were reloaded
        if transport.as_ref().map(|(g, _)| *g) != Some(cfg.generation) {
            match create_mailer(&cfg).await {
                Ok(t) => transport = Some((cfg.generation, t)),
                Err(err) => {
                    transport = None;
                    error!(
                        "Cannot reach the SMTP server, dropping the E-Mail to '{}': {}",
                        req.address, err.message
                    );
                    if let Err(err) = Event::email_send_err(req.typ, &req.address).send().await {
                        error!(?err, "Could not push EmailSendError event");
                    }
                    continue;
                }
            }
        }
        let Some((_, mailer)) = transport.as_mut() else {
            continue;
        };

        let from: Mailbox = match cfg.from.parse() {
            Ok(f) => f,
            Err(err) => {
                error!("Invalid SMTP sender '{}': {err}", cfg.from);
                continue;
            }
        };
        let (typ, address) = (req.typ, req.address.clone());
        let message = match build_message(&from, req) {
            Ok(m) => m,
            Err(err) => {
                error!("Error building the E-Mail to '{address}': {err}");
                continue;
            }
        };

        match mailer.send(message.clone()).await {
            Ok(_) => {
                info!("E-Mail to '{address}' sent successfully!");
                callback.call().await;
                continue;
            }
            Err(err) => {
                error!(error = ?err, "Could not send E-Mail - establishing new connection and retry");
            }
        }

        // short timeout if sending fails, maybe the network just had a short hiccup
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Only try to recreate the connection if we use XOAUTH2, because it uses expiring tokens
        // for authentication. Normal SMTP connections should handle reconnects automatically.
        if vars.smtp_conn_mode == SmtpConnMode::XOauth2
            && let Ok(t) = create_mailer(&cfg).await
        {
            *mailer = t;
        }

        match mailer.send(message).await {
            Ok(_) => {
                info!("E-Mail to '{address}' sent successfully after retry!");
                callback.call().await;
            }
            Err(err) => {
                // Log loudly and emit an event so admins see the failure.
                error!(
                    error = ?err,
                    "Could not send E-Mail to '{address}' even after retrying - dropping it"
                );
                if let Err(err) = Event::email_send_err(typ, &address).send().await {
                    error!(?err, "Could not push EmailSendError event");
                }
            }
        }
    }
}

/// Sends a test E-Mail right now with the current settings and reports the real result.
/// Used by the admin UI, so it does not go through the queue.
pub async fn send_test(to: &str, recipient_name: &str) -> Result<(), ErrorResponse> {
    let cfg = settings::smtp();
    if !cfg.is_configured() {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "No SMTP server is configured",
        ));
    }
    let mailer = create_mailer(&cfg).await?;
    let from: Mailbox = cfg.from.parse().map_err(|err| {
        ErrorResponse::new(
            ErrorResponseType::BadRequest,
            format!("Invalid sender '{}': {err}", cfg.from),
        )
    })?;

    let req = EMail {
        typ: EmailType::Custom,
        recipient_name: recipient_name.to_string(),
        address: to.to_string(),
        subject: format!("{} - SMTP test", RauthyConfig::get().vars.email.sub_prefix),
        text: Some(
            "This is a test message from your Rauthy instance. Your SMTP settings work.".into(),
        ),
        html: None,
    };
    let message = build_message(&from, req)
        .map_err(|err| ErrorResponse::new(ErrorResponseType::BadRequest, err))?;
    mailer.send(message).await.map_err(|err| {
        ErrorResponse::new(
            ErrorResponseType::Connection,
            format!("The SMTP server rejected the message: {err}"),
        )
    })?;
    Ok(())
}

/// Connects to SMTP with a single attempt. The next E-Mail tries again if it fails.
async fn create_mailer(
    cfg: &EffectiveSmtp,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, ErrorResponse> {
    let url = cfg.url.as_deref().unwrap_or_default();
    if cfg.tls_mode == EmailTlsMode::DangerInsecure {
        conn_test_smtp_insecure(url, cfg.port).await
    } else {
        connect_test_smtp(cfg).await
    }
}

fn smtp_err(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(ErrorResponseType::Connection, msg.into())
}

#[tracing::instrument(level = "debug", skip(cfg))]
async fn connect_test_smtp(
    cfg: &EffectiveSmtp,
) -> Result<AsyncSmtpTransport<lettre::Tokio1Executor>, ErrorResponse> {
    let vars = &RauthyConfig::get().vars.email;
    let smtp_url = cfg.url.as_deref().unwrap_or_default();
    let username = cfg
        .username
        .as_deref()
        .ok_or_else(|| smtp_err("The SMTP username is not set"))?
        .trim()
        .to_string();

    let mut mechanisms = Vec::with_capacity(2);
    let creds = if vars.smtp_conn_mode == SmtpConnMode::XOauth2 {
        mechanisms.push(Mechanism::Xoauth2);

        let token = SmtpOauthToken::get().await.map_err(|_| {
            smtp_err("Could not retrieve a `client_credentials` token for SMTP XOAUTH2")
        })?;
        authentication::Credentials::new(username, token.access_token)
    } else {
        mechanisms.push(Mechanism::Plain);
        mechanisms.push(Mechanism::Login);

        let password = cfg
            .password
            .as_deref()
            .ok_or_else(|| smtp_err("The SMTP password is not set"))?
            .trim()
            .to_string();
        authentication::Credentials::new(username, password)
    };

    let mut builder = if cfg.tls_mode == EmailTlsMode::Tls {
        AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(smtp_url)
    } else {
        AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(smtp_url)
    }
    .map_err(|err| smtp_err(format!("Invalid SMTP server '{smtp_url}': {err}")))?;

    if let Some(port) = cfg.port {
        builder = builder.port(port);
    }

    if let Some(root_ca) = &vars.root_ca {
        let cert = client::Certificate::from_pem(root_ca.as_bytes())
            .map_err(|err| smtp_err(format!("Invalid `email.root_ca`: {err}")))?;

        let params = client::TlsParameters::builder(smtp_url.to_string())
            .add_root_certificate(cert)
            .build_rustls()
            .map_err(|err| {
                smtp_err(format!(
                    "Cannot build TLS parameters with `email.root_ca`: {err}"
                ))
            })?;

        builder = builder.tls(client::Tls::Required(params));
    }

    let conn = builder
        .authentication(mechanisms.clone())
        .credentials(creds.clone())
        .timeout(Some(Duration::from_secs(10)))
        .build();

    match conn.test_connection().await {
        Ok(true) => {
            info!(
                "Successfully connected to {} via {}",
                smtp_url, cfg.tls_mode
            );
            Ok(conn)
        }
        Ok(false) => Err(smtp_err(format!(
            "Connected to {smtp_url} via {}, but the SMTP test failed",
            cfg.tls_mode
        ))),
        Err(err) => Err(smtp_err(format!(
            "Could not connect to {smtp_url} via {}: {err}. Check the address, port and credentials",
            cfg.tls_mode
        ))),
    }
}

async fn conn_test_smtp_insecure(
    smtp_url: &str,
    smtp_port: Option<u16>,
) -> Result<AsyncSmtpTransport<lettre::Tokio1Executor>, ErrorResponse> {
    let port = smtp_port.unwrap_or(1025);

    let conn = AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(smtp_url)
        .port(port)
        .build();
    match conn.test_connection().await {
        Ok(true) => {
            info!(
                smtp_url,
                port, "Successfully connected to INSECURE SMTP relay ",
            );
            Ok(conn)
        }
        Ok(false) => Err(smtp_err(format!(
            "Could not connect to the insecure SMTP relay {smtp_url}:{port}"
        ))),
        Err(err) => Err(smtp_err(format!(
            "Could not connect to the insecure SMTP relay {smtp_url}:{port}: {err}"
        ))),
    }
}
