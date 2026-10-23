//! Message channels (email, SMS, WhatsApp) and the drivers that deliver them.
//!
//! Rauthy talks to every external notification agent through ONE contract that is defined here,
//! so swapping the agent only means changing the config:
//!
//! - `smtp`: the built-in mailer, email only (the default for email)
//! - `webhook`: generic. POSTs the [`ChannelMessage`] contract as JSON to `url`, signed with
//!   `X-Rauthy-Signature: sha256=<hex hmac of the raw body>` if an `api_key` is set. Any agent can
//!   be adapted to this with a few lines of glue.
//! - `notifyd`: thin adapter for <https://github.com/rmzlb/notifyd> (`POST {url}/v1/send`)
//!
//! ```toml
//! [channels.sms]
//! driver = "notifyd"            # "" = disabled
//! url = "http://127.0.0.1:3400"
//! api_key = "sk_myapp_..."
//! channel_name = "sms"          # name the agent knows this channel by
//! ```

use crate::email::mailer::{EMail, EmailType};
use crate::email::mailer_callback::EMailCallback;
use crate::rauthy_config::{RauthyConfig, t_str, t_table};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Serialize;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDriver {
    Disabled,
    Smtp,
    Notifyd,
    Webhook,
}

impl ChannelDriver {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "",
            Self::Smtp => "smtp",
            Self::Notifyd => "notifyd",
            Self::Webhook => "webhook",
        }
    }

    fn parse(channel: MessageChannel, s: &str) -> Self {
        match (s.trim().to_lowercase().as_str(), channel) {
            ("", MessageChannel::Email) | ("smtp", MessageChannel::Email) => Self::Smtp,
            ("", _) | ("none", _) | ("disabled", _) => Self::Disabled,
            ("notifyd", _) => Self::Notifyd,
            ("webhook", _) => Self::Webhook,
            (other, _) => panic!(
                "Invalid driver '{other}' for channel {}: expected one of: smtp (email only) \
                notifyd webhook, or empty to disable",
                channel.as_str()
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub driver: ChannelDriver,
    pub url: Option<String>,
    /// notifyd: `X-Api-Key`, webhook: HMAC signing secret
    pub api_key: Option<String>,
    /// the name the agent knows this channel by
    pub channel_name: String,
}

#[derive(Debug)]
pub struct VarsChannels {
    pub email: ChannelConfig,
    pub sms: ChannelConfig,
    pub whatsapp: ChannelConfig,
}

impl Default for VarsChannels {
    fn default() -> Self {
        let cfg = |driver, name: &str| ChannelConfig {
            driver,
            url: None,
            api_key: None,
            channel_name: name.to_string(),
        };
        Self {
            email: cfg(ChannelDriver::Smtp, "email"),
            sms: cfg(ChannelDriver::Disabled, "sms"),
            whatsapp: cfg(ChannelDriver::Disabled, "whatsapp"),
        }
    }
}

impl VarsChannels {
    pub fn get(&self, channel: MessageChannel) -> &ChannelConfig {
        match channel {
            MessageChannel::Email => &self.email,
            MessageChannel::Sms => &self.sms,
            MessageChannel::Whatsapp => &self.whatsapp,
        }
    }

    /// Channels that can currently be used for sending.
    pub fn available(&self) -> Vec<(MessageChannel, ChannelDriver)> {
        [
            MessageChannel::Email,
            MessageChannel::Sms,
            MessageChannel::Whatsapp,
        ]
        .into_iter()
        .map(|c| (c, self.get(c).driver))
        .filter(|(_, d)| *d != ChannelDriver::Disabled)
        .collect()
    }

    pub fn parse(&mut self, table: &mut toml::Table) {
        let mut table = t_table(table, "channels");
        for (key, channel, prefix) in [
            ("email", MessageChannel::Email, "CHANNEL_EMAIL"),
            ("sms", MessageChannel::Sms, "CHANNEL_SMS"),
            ("whatsapp", MessageChannel::Whatsapp, "CHANNEL_WHATSAPP"),
        ] {
            let mut t = t_table(&mut table, key);
            let parent = format!("channels.{key}");
            let cfg = match channel {
                MessageChannel::Email => &mut self.email,
                MessageChannel::Sms => &mut self.sms,
                MessageChannel::Whatsapp => &mut self.whatsapp,
            };

            if let Some(v) = t_str(&mut t, &parent, "driver", &format!("{prefix}_DRIVER")) {
                cfg.driver = ChannelDriver::parse(channel, &v);
            }
            if let Some(v) = t_str(&mut t, &parent, "url", &format!("{prefix}_URL")) {
                cfg.url = Some(v.trim_end_matches('/').to_string());
            }
            if let Some(v) = t_str(&mut t, &parent, "api_key", &format!("{prefix}_API_KEY")) {
                cfg.api_key = Some(v);
            }
            if let Some(v) = t_str(&mut t, &parent, "channel_name", &format!("{prefix}_NAME")) {
                cfg.channel_name = v;
            }

            if matches!(cfg.driver, ChannelDriver::Notifyd | ChannelDriver::Webhook)
                && cfg.url.is_none()
            {
                panic!("channels.{key}: a `url` is required for the driver");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePurpose {
    /// manual message of an admin, bulk
    Admin,
    /// verification codes, time-critical
    Verification,
    /// security notifications
    Security,
}

/// The one contract between Rauthy and every notification agent. Version 1.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelMessage {
    pub version: u8,
    pub channel: &'static str,
    pub purpose: MessagePurpose,
    pub recipient: ChannelRecipient,
    pub subject: Option<String>,
    pub text: String,
    pub html: Option<String>,
    /// stable key, so a retry never delivers twice
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelRecipient {
    pub user_id: String,
    pub name: String,
    /// email address or E.164 phone number, depending on the channel
    pub to: String,
}

impl ChannelMessage {
    pub fn new(
        channel: MessageChannel,
        purpose: MessagePurpose,
        recipient: ChannelRecipient,
        subject: Option<String>,
        text: String,
        html: Option<String>,
        idempotency_key: Option<String>,
    ) -> Self {
        Self {
            version: 1,
            channel: channel.as_str(),
            purpose,
            recipient,
            subject,
            text,
            html,
            idempotency_key,
        }
    }
}

/// Sends a plain-text email through the built-in mailer to ANY address (e.g. the address of a
/// verifiable user attribute), unlike `email::custom::send_custom`, which mails the user's own.
pub async fn send_email_to(
    address: &str,
    recipient_name: &str,
    subject: &str,
    text: String,
) -> Result<(), ErrorResponse> {
    let prefix = &RauthyConfig::get().vars.email.sub_prefix;
    let req = EMail {
        typ: EmailType::Custom,
        recipient_name: recipient_name.to_string(),
        address: address.to_string(),
        subject: format!("{prefix} - {subject}"),
        text: Some(text),
        html: None,
    };
    RauthyConfig::get()
        .tx_email
        .send_timeout((req, EMailCallback::None), Duration::from_secs(10))
        .await
        .map_err(|_| ErrorResponse::new(ErrorResponseType::Timeout, "email send timeout"))
}

/// Checks that the agent of a channel can be reached. Returns a short description of what was
/// checked. Nothing is delivered to a user.
pub async fn test(channel: MessageChannel) -> Result<String, ErrorResponse> {
    let cfg = crate::settings::channel(channel);
    let url = cfg.url.as_deref().unwrap_or_default();

    let req = match cfg.driver {
        ChannelDriver::Notifyd => CLIENT.get(format!("{url}/v1/health")),
        ChannelDriver::Webhook => {
            let body = br#"{"version":1,"type":"ping"}"#.to_vec();
            let mut r = CLIENT.post(url).header("content-type", "application/json");
            if let Some(k) = &cfg.api_key {
                r = r.header("x-rauthy-signature", webhook_signature(&body, k));
            }
            r.body(body)
        }
        ChannelDriver::Smtp => {
            return Err(ErrorResponse::new(
                ErrorResponseType::BadRequest,
                "The smtp driver is tested with the SMTP test",
            ));
        }
        ChannelDriver::Disabled => {
            return Err(ErrorResponse::new(
                ErrorResponseType::BadRequest,
                "The channel is disabled",
            ));
        }
    };

    let res = req.send().await.map_err(|err| {
        ErrorResponse::new(
            ErrorResponseType::Connection,
            format!("The agent is not reachable: {err}"),
        )
    })?;
    let status = res.status();
    if !status.is_success() {
        return Err(ErrorResponse::new(
            ErrorResponseType::Connection,
            format!("The agent answered {status}"),
        ));
    }
    Ok(format!("The agent answered {status}"))
}

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    // The agent URL is trusted admin config (often a private / docker address), so this
    // deliberately does not use the SSRF-guarded global client.
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("rauthy/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("channels HTTP client")
});

/// Request body for the notifyd `POST /v1/send` endpoint.
fn notifyd_body(msg: &ChannelMessage, channel_name: &str) -> serde_json::Value {
    let (priority, transactional) = match msg.purpose {
        MessagePurpose::Admin => ("bulk", false),
        MessagePurpose::Verification => ("critical", true),
        MessagePurpose::Security => ("high", true),
    };
    let body = msg.html.as_deref().unwrap_or(&msg.text);

    let mut v = serde_json::json!({
        "channel": channel_name,
        "to": msg.recipient.to,
        "body": body,
        "priority": priority,
        "transactional": transactional,
    });
    if let Some(s) = &msg.subject {
        v["subject"] = s.as_str().into();
    }
    if let Some(k) = &msg.idempotency_key {
        v["idempotency_key"] = k.as_str().into();
    }
    v
}

/// `sha256=<hex>` HMAC of the raw webhook body
fn webhook_signature(body: &[u8], secret: &str) -> String {
    format!(
        "sha256={}",
        hex::encode(hmac_sha256::HMAC::mac(body, secret.as_bytes()))
    )
}

/// Delivers a message through the driver configured for its channel.
/// Returns the agent's job id, if it gave one.
///
/// The `smtp` driver is handled by the mailer itself and cannot be used here.
pub async fn send(
    channel: MessageChannel,
    msg: &ChannelMessage,
) -> Result<Option<String>, ErrorResponse> {
    // DB settings first, config file as the fallback
    let cfg = crate::settings::channel(channel);
    let cfg = &cfg;
    let url = cfg.url.as_deref().unwrap_or_default();

    let req = match cfg.driver {
        ChannelDriver::Notifyd => {
            let body = notifyd_body(msg, &cfg.channel_name).to_string();
            let mut r = CLIENT
                .post(format!("{url}/v1/send"))
                .header("content-type", "application/json")
                .body(body);
            if let Some(k) = &cfg.api_key {
                r = r.header("x-api-key", k);
            }
            r
        }
        ChannelDriver::Webhook => {
            let body = serde_json::to_vec(msg).unwrap_or_default();
            let mut r = CLIENT.post(url).header("content-type", "application/json");
            if let Some(k) = &cfg.api_key {
                r = r.header("x-rauthy-signature", webhook_signature(&body, k));
            }
            r.body(body)
        }
        ChannelDriver::Smtp | ChannelDriver::Disabled => {
            return Err(ErrorResponse::new(
                ErrorResponseType::BadRequest,
                format!("Channel '{}' has no external driver", channel.as_str()),
            ));
        }
    };

    let res = req.send().await.map_err(|err| {
        ErrorResponse::new(
            ErrorResponseType::Connection,
            format!("Notification agent not reachable: {err}"),
        )
    })?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ErrorResponse::new(
            ErrorResponseType::Connection,
            format!("Notification agent answered {status}: {text}"),
        ));
    }
    debug!(
        "channel {} delivered via agent: {}",
        channel.as_str(),
        status
    );

    // notifyd: {"job_ids": [...]}, generic: {"job_id": "..."}
    let id = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| {
            v.get("job_id")
                .and_then(|j| j.as_str().map(String::from))
                .or_else(|| {
                    v.get("job_ids")
                        .and_then(|j| j.get(0))
                        .and_then(|j| j.as_str().map(String::from))
                })
        });
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(purpose: MessagePurpose) -> ChannelMessage {
        ChannelMessage::new(
            MessageChannel::Sms,
            purpose,
            ChannelRecipient {
                user_id: "u1".into(),
                name: "Alice".into(),
                to: "+256700000001".into(),
            },
            None,
            "Your code is 123456".into(),
            None,
            Some("verify-u1-1".into()),
        )
    }

    #[test]
    fn notifyd_payload_maps_contract() {
        let v = notifyd_body(&msg(MessagePurpose::Verification), "sms");
        assert_eq!(v["channel"], "sms");
        assert_eq!(v["to"], "+256700000001");
        assert_eq!(v["body"], "Your code is 123456");
        assert_eq!(v["priority"], "critical");
        assert_eq!(v["transactional"], true);
        assert_eq!(v["idempotency_key"], "verify-u1-1");
        assert!(v.get("subject").is_none());

        let v = notifyd_body(&msg(MessagePurpose::Admin), "sms_provider_x");
        assert_eq!(v["channel"], "sms_provider_x");
        assert_eq!(v["priority"], "bulk");
        assert_eq!(v["transactional"], false);
    }

    #[test]
    fn webhook_signature_is_stable() {
        // HMAC-SHA256("body", key "secret")
        assert_eq!(
            webhook_signature(b"body", "secret"),
            webhook_signature(b"body", "secret")
        );
        assert!(webhook_signature(b"body", "secret").starts_with("sha256="));
        assert_ne!(
            webhook_signature(b"body", "secret"),
            webhook_signature(b"body2", "secret")
        );
    }

    #[test]
    fn driver_defaults() {
        assert_eq!(
            ChannelDriver::parse(MessageChannel::Email, ""),
            ChannelDriver::Smtp
        );
        assert_eq!(
            ChannelDriver::parse(MessageChannel::Sms, ""),
            ChannelDriver::Disabled
        );
        assert_eq!(
            ChannelDriver::parse(MessageChannel::Sms, "NotifyD"),
            ChannelDriver::Notifyd
        );
    }
}
