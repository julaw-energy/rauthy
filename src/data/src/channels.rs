//! Message channels and the drivers that deliver them.
//!
//! `email` and `sms` always exist, an admin can add more (see [`crate::settings`]). A channel
//! delivers through one of three drivers, so nothing here has to change for a new provider:
//!
//! - `smtp`: email through an SMTP server. For the channel `email` that is the server under
//!   Config -> Email (SMTP); a custom channel has its own server and credentials.
//! - `webhook`: POSTs the [`ChannelMessage`] contract as JSON to `url`. With a signing secret
//!   set, the request carries `X-Rauthy-Signature: sha256=<hex hmac of the raw body>` and, to
//!   stop replays, `X-Rauthy-Timestamp: <unix seconds>` with
//!   `X-Rauthy-Signature-Timestamped: sha256=<hex hmac of "<timestamp>.<raw body>">`. Every
//!   request also has an `Idempotency-Key` header (the same as in the body) that stays the same
//!   for a retry. A notification agent only has to understand this one format.
//! - `http`: an admin describes the call to any provider: method, URL, headers and body
//!   attributes, each of which can be marked secret. Values can contain `{{to}}`, `{{text}}`, ...
//!
//! A channel can also carry an expiry date, after which it stops delivering.
//!
//! ```toml
//! [channels.sms]
//! driver = "webhook"            # "" = disabled
//! url = "https://agent.example.com/hook"
//! api_key = "signing secret"
//! ```
//! (`http` channels and custom channels are managed in the admin UI only.)

use crate::email::mailer::{self, EMail, EmailType};
use crate::email::mailer_callback::EMailCallback;
use crate::entity::user_attr::UserAttrValueEntity;
use crate::entity::users::User;
use crate::entity::users_values::UserValues;
use crate::message_templates::fill;
use crate::rauthy_config::{RauthyConfig, t_str, t_table};
use crate::settings::EffectiveSmtp;
use chrono::Utc;
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_common::utils::get_rand;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Serialize;
use serde_json::Value;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDriver {
    Disabled,
    Smtp,
    Webhook,
    Http,
}

impl ChannelDriver {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "",
            Self::Smtp => "smtp",
            Self::Webhook => "webhook",
            Self::Http => "http",
        }
    }

    /// `None` for a value that is no driver
    pub fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "none" | "disabled" => Some(Self::Disabled),
            "smtp" => Some(Self::Smtp),
            "webhook" => Some(Self::Webhook),
            "http" => Some(Self::Http),
            _ => None,
        }
    }

    fn parse(channel: &MessageChannel, s: &str) -> Self {
        match Self::from_name(s) {
            Some(Self::Smtp) if !channel.is_email() => panic!(
                "channels.{channel}: the driver `smtp` is only available for the email channel \
                in the config file"
            ),
            Some(Self::Http) => {
                panic!("channels.{channel}: the driver `http` can only be set up in the admin UI")
            }
            Some(d) => d,
            None => panic!(
                "Invalid driver '{s}' for channel {channel}: expected one of: smtp (email only) \
                webhook, or empty to disable. The `notifyd` driver is gone, use `webhook`."
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFormat {
    Json,
    Form,
}

impl BodyFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Form => "form",
        }
    }
}

/// A header or a body attribute of an `http` channel. `value` of a secret is decrypted.
#[derive(Debug, Clone)]
pub struct HttpKv {
    pub name: String,
    pub value: String,
    pub secret: bool,
}

/// How an `http` channel calls its provider, with secrets decrypted once when the settings are
/// loaded, not for every message.
#[derive(Debug, Clone)]
pub struct HttpRuntime {
    pub method: String,
    pub url: String,
    pub headers: Vec<HttpKv>,
    pub body_format: BodyFormat,
    pub body: Vec<HttpKv>,
}

#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub driver: ChannelDriver,
    /// `webhook`
    pub url: Option<String>,
    /// `webhook`: the HMAC signing secret
    pub api_key: Option<String>,
    /// shown in the UI
    pub label: String,
    /// The value of a user this channel delivers to: `email`, `phone` or `attr:<name>`
    pub recipient: String,
    /// Unix timestamp, custom channels only
    pub expires: Option<i64>,
    /// `smtp` of a custom channel
    pub smtp: Option<Arc<EffectiveSmtp>>,
    pub http: Option<Arc<HttpRuntime>>,
}

impl ChannelConfig {
    pub fn expired(&self) -> bool {
        self.expires.is_some_and(|e| e <= Utc::now().timestamp())
    }

    /// `true` if messages can go out through the channel right now
    pub fn is_usable(&self) -> bool {
        self.driver != ChannelDriver::Disabled && !self.expired()
    }

    /// Whether HTML content can be sent through it: the mailer and its own-server copies
    pub fn supports_html(&self, channel: &MessageChannel) -> bool {
        channel.is_email() || self.driver == ChannelDriver::Smtp
    }

    fn plain(driver: ChannelDriver, label: &str, recipient: &str) -> Self {
        Self {
            driver,
            url: None,
            api_key: None,
            label: label.to_string(),
            recipient: recipient.to_string(),
            expires: None,
            smtp: None,
            http: None,
        }
    }

    /// A channel that does not exist
    pub fn missing(channel: &MessageChannel) -> Self {
        Self::plain(
            ChannelDriver::Disabled,
            channel.as_str(),
            if channel.is_email() { "email" } else { "phone" },
        )
    }
}

#[derive(Debug)]
pub struct VarsChannels {
    pub email: ChannelConfig,
    pub sms: ChannelConfig,
}

impl Default for VarsChannels {
    fn default() -> Self {
        Self {
            email: ChannelConfig::plain(ChannelDriver::Smtp, "Email", "email"),
            sms: ChannelConfig::plain(ChannelDriver::Disabled, "SMS", "phone"),
        }
    }
}

impl VarsChannels {
    /// The config-file values of a built-in channel. Custom channels only exist in the DB.
    pub fn get(&self, channel: &MessageChannel) -> Option<&ChannelConfig> {
        match channel.as_str() {
            "email" => Some(&self.email),
            "sms" => Some(&self.sms),
            _ => None,
        }
    }

    pub fn parse(&mut self, table: &mut toml::Table) {
        let mut table = t_table(table, "channels");
        // there is no built-in whatsapp channel any more, it is created in the admin UI
        let _ = t_table(&mut table, "whatsapp");
        for (key, prefix) in [("email", "CHANNEL_EMAIL"), ("sms", "CHANNEL_SMS")] {
            let channel: MessageChannel = key.parse().expect("built-in channel name");
            let mut t = t_table(&mut table, key);
            let parent = format!("channels.{key}");
            let cfg = if key == "email" {
                &mut self.email
            } else {
                &mut self.sms
            };

            if let Some(v) = t_str(&mut t, &parent, "driver", &format!("{prefix}_DRIVER")) {
                cfg.driver = ChannelDriver::parse(&channel, &v);
            }
            if let Some(v) = t_str(&mut t, &parent, "url", &format!("{prefix}_URL")) {
                cfg.url = Some(v.trim_end_matches('/').to_string());
            }
            if let Some(v) = t_str(&mut t, &parent, "api_key", &format!("{prefix}_API_KEY")) {
                cfg.api_key = Some(v);
            }

            if cfg.driver == ChannelDriver::Webhook && cfg.url.is_none() {
                panic!("channels.{key}: a `url` is required for the webhook driver");
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
    pub channel: String,
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
        channel: &MessageChannel,
        purpose: MessagePurpose,
        recipient: ChannelRecipient,
        subject: Option<String>,
        text: String,
        html: Option<String>,
        idempotency_key: Option<String>,
    ) -> Self {
        Self {
            version: 1,
            channel: channel.as_str().to_string(),
            purpose,
            recipient,
            subject,
            text,
            html,
            // a retry of this message carries the same key, so an agent can drop duplicates
            idempotency_key: idempotency_key.or_else(|| Some(get_rand(24))),
        }
    }
}

/// `true` for `email`, `phone` or `attr:<name>`. Whether the attribute exists is checked where
/// the source is saved.
pub fn recipient_source_is_valid(s: &str) -> bool {
    s == "email"
        || s == "phone"
        || s.strip_prefix("attr:")
            .is_some_and(|n| !n.is_empty() && n.len() <= 64 && !n.contains(char::is_whitespace))
}

/// The value of the user a channel with this recipient source delivers to. `None` if the user
/// has none, in which case nothing may be sent through the channel.
pub async fn recipient_value(user: &User, source: &str) -> Result<Option<String>, ErrorResponse> {
    match source {
        "email" => Ok(Some(user.email.clone()).filter(|e| !e.is_empty())),
        "phone" => Ok(UserValues::find(&user.id)
            .await?
            .and_then(|v| v.phone)
            .filter(|p| !p.trim().is_empty())),
        other => {
            let Some(name) = other.strip_prefix("attr:") else {
                return Ok(None);
            };
            Ok(UserAttrValueEntity::find_for_user(&user.id)
                .await?
                .into_iter()
                .find(|v| v.key == name)
                .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v.value).ok())
                .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|v| !v.is_empty()))
        }
    }
}

/// Like [`recipient_value`], but a missing value is an error: the message must be rejected.
pub async fn resolve_recipient(user: &User, source: &str) -> Result<String, ErrorResponse> {
    recipient_value(user, source).await?.ok_or_else(|| {
        let what = match source {
            "email" => "email address".to_string(),
            "phone" => "phone number".to_string(),
            other => format!("value for '{}'", other.trim_start_matches("attr:")),
        };
        ErrorResponse::new(
            ErrorResponseType::BadRequest,
            format!("The user has no {what}"),
        )
    })
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

fn err_cfg(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(ErrorResponseType::BadRequest, msg.into())
}

fn not_usable(channel: &MessageChannel, cfg: &ChannelConfig) -> ErrorResponse {
    if cfg.expired() {
        err_cfg(format!("The channel '{channel}' has expired"))
    } else {
        err_cfg(format!("The channel '{channel}' is not configured"))
    }
}

/// Delivers a plain text message to `to` through the channel, whatever its driver is.
pub async fn deliver(
    channel: &MessageChannel,
    user: &User,
    to: String,
    purpose: MessagePurpose,
    subject: &str,
    text: String,
) -> Result<Delivery, ErrorResponse> {
    let cfg = crate::settings::channel(channel);
    if !cfg.is_usable() {
        return Err(not_usable(channel, &cfg));
    }
    if cfg.driver == ChannelDriver::Smtp && channel.is_email() {
        // the default mailer, queued like every other system email
        return send_email_to(&to, &user.email_recipient_name(), subject, text)
            .await
            .map(|_| Delivery::default());
    }

    let msg = ChannelMessage::new(
        channel,
        purpose,
        ChannelRecipient {
            user_id: user.id.clone(),
            name: user.email_recipient_name(),
            to,
        },
        // plain text channels like SMS have no subject
        Some(subject.to_string()).filter(|_| {
            channel.is_email() || matches!(cfg.driver, ChannelDriver::Smtp | ChannelDriver::Http)
        }),
        text,
        None,
        None,
    );
    let cfg = crate::settings::channel(channel);
    send_with_report(channel, &cfg, &msg).await
}

/// What the other side of a channel answered
#[derive(Debug, Clone, Default)]
pub struct Delivery {
    /// HTTP status of the agent / provider, none for SMTP
    pub status: Option<u16>,
    /// `job_id` of the answer, if it had one
    pub id: Option<String>,
    /// the start of the answer body, for the admin who tests a channel
    pub answer: String,
}

/// A system email (password reset, one-time code, ...) through the `email` channel when that
/// does not use the built-in SMTP mailer. The caller has applied the branding already.
pub async fn send_system_email(
    cfg: &ChannelConfig,
    req: &EMail,
    idempotency_key: &str,
) -> Result<(), ErrorResponse> {
    let channel = MessageChannel::email();
    let msg = ChannelMessage::new(
        &channel,
        MessagePurpose::Security,
        ChannelRecipient {
            user_id: String::new(),
            name: req.recipient_name.clone(),
            to: req.address.clone(),
        },
        Some(req.subject.clone()),
        req.text.clone().unwrap_or_default(),
        req.html.clone(),
        Some(idempotency_key.to_string()),
    );
    send_with(&channel, cfg, &msg).await.map(|_| ())
}

/// Checks that the other side of a channel can be reached. Returns a short description of what
/// was checked. Nothing is delivered to a user.
pub async fn test(channel: &MessageChannel) -> Result<String, ErrorResponse> {
    let cfg = crate::settings::channel(channel);

    match cfg.driver {
        ChannelDriver::Webhook => {
            let url = cfg.url.as_deref().unwrap_or_default();
            let body = br#"{"version":1,"type":"ping"}"#.to_vec();
            let mut r = CLIENT.post(url).header("content-type", "application/json");
            if let Some(k) = &cfg.api_key {
                r = r.header("x-rauthy-signature", webhook_signature(&body, k));
            }
            let res = r.body(body).send().await.map_err(|err| {
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
        ChannelDriver::Smtp if !channel.is_email() => {
            let smtp = cfg
                .smtp
                .as_deref()
                .ok_or_else(|| err_cfg("No SMTP server is set up for this channel"))?;
            mailer::test_connection(smtp).await?;
            Ok("Connected to the SMTP server".to_string())
        }
        ChannelDriver::Smtp => Err(err_cfg("The default mailer is tested under Email (SMTP)")),
        ChannelDriver::Http => Err(err_cfg(
            "An http channel has nothing to ping, use \"Send a test to me\"",
        )),
        ChannelDriver::Disabled => Err(err_cfg("The channel is disabled")),
    }
}

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    // The URL is trusted admin config (often a private / docker address), so this deliberately
    // does not use the SSRF-guarded global client.
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("rauthy/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("channels HTTP client")
});

/// `sha256=<hex>` HMAC of the raw webhook body
fn webhook_signature(body: &[u8], secret: &str) -> String {
    format!(
        "sha256={}",
        hex::encode(hmac_sha256::HMAC::mac(body, secret.as_bytes()))
    )
}

/// `sha256=<hex>` HMAC of `<unix seconds>.<raw body>`. The receiver rejects a timestamp that is
/// too old, which stops a captured request from being replayed.
fn webhook_signature_timestamped(body: &[u8], timestamp: i64, secret: &str) -> String {
    let mut signed = format!("{timestamp}.").into_bytes();
    signed.extend_from_slice(body);
    webhook_signature(&signed, secret)
}

fn purpose_str(p: MessagePurpose) -> &'static str {
    match p {
        MessagePurpose::Admin => "admin",
        MessagePurpose::Verification => "verification",
        MessagePurpose::Security => "security",
    }
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Sets `value` at a dotted path like `messages.0.to`: a number is an array index, anything else
/// an object key.
fn set_path(root: &mut Value, path: &str, value: Value) {
    let mut value = Some(value);
    let segs: Vec<&str> = path.split('.').collect();
    let mut cur = root;
    for (i, seg) in segs.iter().enumerate() {
        match seg.parse::<usize>() {
            Ok(n) => {
                if !cur.is_array() {
                    *cur = Value::Array(Vec::new());
                }
                let arr = cur.as_array_mut().expect("array");
                while arr.len() <= n {
                    arr.push(Value::Null);
                }
                cur = &mut arr[n];
            }
            Err(_) => {
                if !cur.is_object() {
                    *cur = Value::Object(Default::default());
                }
                cur = cur
                    .as_object_mut()
                    .expect("object")
                    .entry(seg.to_string())
                    .or_insert(Value::Null);
            }
        }
        if i == segs.len() - 1 {
            *cur = value.take().unwrap_or(Value::Null);
        }
    }
}

/// The values a message offers to the `{{...}}` of an `http` channel
fn message_vars(msg: &ChannelMessage, app_name: &str) -> Vec<(&'static str, String)> {
    vec![
        ("to", msg.recipient.to.clone()),
        ("name", msg.recipient.name.clone()),
        ("text", msg.text.clone()),
        ("html", msg.html.clone().unwrap_or_default()),
        ("subject", msg.subject.clone().unwrap_or_default()),
        ("purpose", purpose_str(msg.purpose).to_string()),
        ("channel", msg.channel.clone()),
        ("app_name", app_name.to_string()),
        (
            "idempotency_key",
            msg.idempotency_key.clone().unwrap_or_default(),
        ),
    ]
}

/// Builds the request an `http` channel makes for a message
fn http_request(
    cfg: &HttpRuntime,
    msg: &ChannelMessage,
    app_name: &str,
) -> Result<reqwest::RequestBuilder, ErrorResponse> {
    let vars = message_vars(msg, app_name);
    let plain: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let encoded_vars: Vec<(&str, String)> =
        vars.iter().map(|(k, v)| (*k, percent_encode(v))).collect();
    let encoded: Vec<(&str, &str)> = encoded_vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
    // secrets are used literally, everything else may contain placeholders
    let value_of = |kv: &HttpKv| {
        if kv.secret {
            kv.value.clone()
        } else {
            fill(&kv.value, &plain)
        }
    };

    let method = reqwest::Method::from_bytes(cfg.method.as_bytes())
        .map_err(|_| err_cfg("Invalid HTTP method"))?;
    let mut req = CLIENT.request(method, fill(&cfg.url, &encoded));

    let mut has_content_type = false;
    for h in &cfg.headers {
        has_content_type |= h.name.eq_ignore_ascii_case("content-type");
        // a line break in a header value would split the header
        let v = value_of(h).replace(['\r', '\n'], " ");
        req = req.header(h.name.as_str(), v);
    }

    match cfg.body_format {
        BodyFormat::Json => {
            let mut root = Value::Null;
            for kv in &cfg.body {
                set_path(&mut root, &kv.name, Value::String(value_of(kv)));
            }
            if root.is_null() {
                root = Value::Object(Default::default());
            }
            if !has_content_type {
                req = req.header("content-type", "application/json");
            }
            req = req.body(root.to_string());
        }
        BodyFormat::Form => {
            let pairs: Vec<(String, String)> = cfg
                .body
                .iter()
                .map(|kv| (kv.name.clone(), value_of(kv)))
                .collect();
            req = req.form(&pairs);
        }
    }
    Ok(req)
}

/// Delivers a message through the driver configured for its channel.
pub async fn send(
    channel: &MessageChannel,
    msg: &ChannelMessage,
) -> Result<Option<String>, ErrorResponse> {
    // DB settings first, config file as the fallback
    let cfg = crate::settings::channel(channel);
    if !cfg.is_usable() {
        return Err(not_usable(channel, &cfg));
    }
    send_with(channel, &cfg, msg).await
}

async fn send_with(
    channel: &MessageChannel,
    cfg: &ChannelConfig,
    msg: &ChannelMessage,
) -> Result<Option<String>, ErrorResponse> {
    send_with_report(channel, cfg, msg).await.map(|d| d.id)
}

async fn send_with_report(
    channel: &MessageChannel,
    cfg: &ChannelConfig,
    msg: &ChannelMessage,
) -> Result<Delivery, ErrorResponse> {
    let req = match cfg.driver {
        ChannelDriver::Webhook => {
            let url = cfg.url.as_deref().unwrap_or_default();
            let body = serde_json::to_vec(msg).unwrap_or_default();
            let mut r = CLIENT.post(url).header("content-type", "application/json");
            if let Some(key) = &msg.idempotency_key {
                r = r.header("idempotency-key", key);
            }
            if let Some(k) = &cfg.api_key {
                let ts = chrono::Utc::now().timestamp();
                r = r
                    .header("x-rauthy-signature", webhook_signature(&body, k))
                    .header("x-rauthy-timestamp", ts)
                    .header(
                        "x-rauthy-signature-timestamped",
                        webhook_signature_timestamped(&body, ts, k),
                    );
            }
            r.body(body)
        }
        ChannelDriver::Http => {
            let http = cfg
                .http
                .as_deref()
                .ok_or_else(|| err_cfg("The http channel has no request configured"))?;
            http_request(http, msg, &crate::settings::app_name())?
        }
        ChannelDriver::Smtp if !channel.is_email() => {
            // a custom channel with its own server
            let smtp = cfg
                .smtp
                .as_deref()
                .ok_or_else(|| err_cfg("No SMTP server is set up for this channel"))?;
            let mail = EMail {
                typ: EmailType::Custom,
                recipient_name: msg.recipient.name.clone(),
                address: msg.recipient.to.clone(),
                subject: msg.subject.clone().unwrap_or_default(),
                text: Some(msg.text.clone()).filter(|t| !t.is_empty()),
                html: msg.html.clone(),
            };
            mailer::send_with(channel.as_str(), smtp, mail).await?;
            return Ok(Delivery::default());
        }
        ChannelDriver::Smtp | ChannelDriver::Disabled => {
            return Err(err_cfg(format!(
                "Channel '{channel}' has no external driver"
            )));
        }
    };

    let res = req.send().await.map_err(|err| {
        ErrorResponse::new(
            ErrorResponseType::Connection,
            format!("The channel's endpoint is not reachable: {err}"),
        )
    })?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ErrorResponse::new(
            ErrorResponseType::Connection,
            format!(
                "The channel's endpoint answered {status}: {}",
                text.chars().take(300).collect::<String>()
            ),
        ));
    }
    debug!("channel {channel} delivered: {status}");

    // a generic answer may carry an id: {"job_id": "..."}
    let id = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("job_id").and_then(|j| j.as_str().map(String::from)));
    Ok(Delivery {
        status: Some(status.as_u16()),
        id,
        answer: text.chars().take(300).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(purpose: MessagePurpose) -> ChannelMessage {
        ChannelMessage::new(
            &MessageChannel::sms(),
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
    fn webhook_signature_is_stable() {
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
    fn timestamped_signature_binds_the_time() {
        let a = webhook_signature_timestamped(b"body", 1000, "secret");
        assert_eq!(a, webhook_signature(b"1000.body", "secret"));
        assert_ne!(a, webhook_signature_timestamped(b"body", 1001, "secret"));
        assert_ne!(a, webhook_signature(b"body", "secret"));
    }

    #[test]
    fn every_message_has_an_idempotency_key() {
        let m = msg(MessagePurpose::Admin);
        assert_eq!(m.idempotency_key.as_deref(), Some("verify-u1-1"));
        let m = ChannelMessage::new(
            &MessageChannel::sms(),
            MessagePurpose::Admin,
            ChannelRecipient {
                user_id: "u".into(),
                name: "n".into(),
                to: "t".into(),
            },
            None,
            "x".into(),
            None,
            None,
        );
        assert!(m.idempotency_key.is_some_and(|k| k.len() >= 16));
    }

    #[test]
    fn driver_names() {
        assert_eq!(ChannelDriver::from_name(""), Some(ChannelDriver::Disabled));
        assert_eq!(ChannelDriver::from_name("HTTP"), Some(ChannelDriver::Http));
        assert_eq!(ChannelDriver::from_name("notifyd"), None);
        assert_eq!(
            ChannelDriver::parse(&MessageChannel::email(), ""),
            ChannelDriver::Disabled
        );
    }

    #[test]
    fn json_body_paths() {
        let mut root = Value::Null;
        set_path(&mut root, "from", "Acme".into());
        set_path(&mut root, "messages.0.to", "+1".into());
        set_path(&mut root, "messages.0.text", "hi".into());
        set_path(&mut root, "messages.1.to", "+2".into());
        assert_eq!(
            root,
            serde_json::json!({
                "from": "Acme",
                "messages": [{"to": "+1", "text": "hi"}, {"to": "+2"}]
            })
        );
    }

    #[test]
    fn percent_encoding() {
        assert_eq!(percent_encode("+256 7&x"), "%2B256%207%26x");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn http_request_is_built_from_the_message() {
        let cfg = HttpRuntime {
            method: "POST".into(),
            url: "https://api.example.com/send?to={{to}}".into(),
            headers: vec![
                HttpKv {
                    name: "Authorization".into(),
                    value: "Bearer {{not_filled}}".into(),
                    secret: true,
                },
                HttpKv {
                    name: "X-Purpose".into(),
                    value: "{{purpose}}".into(),
                    secret: false,
                },
            ],
            body_format: BodyFormat::Json,
            body: vec![HttpKv {
                name: "message.text".into(),
                value: "{{text}} / {{name}}".into(),
                secret: false,
            }],
        };
        let req = http_request(&cfg, &msg(MessagePurpose::Verification), "App")
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            req.url().as_str(),
            "https://api.example.com/send?to=%2B256700000001"
        );
        // secrets are literal
        assert_eq!(req.headers()["authorization"], "Bearer {{not_filled}}");
        assert_eq!(req.headers()["x-purpose"], "verification");
        assert_eq!(req.headers()["content-type"], "application/json");
        let body = std::str::from_utf8(req.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!({"message": {"text": "Your code is 123456 / Alice"}})
        );
    }
}
