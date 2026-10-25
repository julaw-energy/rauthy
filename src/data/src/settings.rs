//! Settings that live in the database instead of the config file.
//!
//! The config file / env stay the base. On the first start with a value in the file and nothing
//! in the DB, the group is *seeded* from the file once; from then on the DB row is authoritative
//! and can be changed from the admin UI without a restart. A changed value is applied on the
//! node that saved it immediately, and on all other nodes within [`WATCH_INTERVAL`].
//!
//! Groups so far: `smtp` (the built-in mailer) and `channel.<email|sms|whatsapp>` (see
//! [`crate::channels`]). Secrets are stored encrypted and never leave the backend.

use crate::channels::{BodyFormat, ChannelConfig, ChannelDriver, HttpKv, HttpRuntime};
use crate::database::DB;
use crate::entity::user_attr::UserAttrConfigEntity;
use crate::rauthy_config::{EmailTlsMode, RauthyConfig, UserValueConfigValue};
use chrono::Utc;
use cryptr::EncValue;
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::settings::{
    ChannelSettingsRequest, ChannelSettingsResponse, HttpKvRequest, HttpKvResponse,
    HttpSettingsRequest, HttpSettingsResponse, LifetimesSettings, LifetimesSettingsResponse,
    SmtpSettingsRequest, SmtpSettingsResponse,
};
use rauthy_common::is_hiqlite;
use rauthy_common::utils::{base64_decode, base64_encode};
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;
use tracing::{error, info, warn};

const WATCH_INTERVAL: Duration = Duration::from_secs(15);

pub const KEY_SMTP: &str = "smtp";
pub const KEY_GENERAL: &str = "general";
pub const KEY_USER_VALUES: &str = "user_values";
pub const KEY_LIFETIMES: &str = "lifetimes";

/// The lifetimes in use, with typed durations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lifetimes {
    pub refresh_token_grace_time: Duration,
    pub refresh_token_lifetime: Duration,
    pub session_lifetime: Duration,
    pub session_timeout: Duration,
    pub session_renew_mfa: bool,
    pub magic_link_pwd_reset: Duration,
    pub magic_link_pwd_first: Duration,
}

impl Lifetimes {
    fn from_file() -> Self {
        let l = &RauthyConfig::get().vars.lifetimes;
        Self {
            refresh_token_grace_time: l.refresh_token_grace_time,
            refresh_token_lifetime: l.refresh_token_lifetime,
            session_lifetime: l.session_lifetime,
            session_timeout: l.session_timeout,
            session_renew_mfa: l.session_renew_mfa,
            magic_link_pwd_reset: l.magic_link_pwd_reset,
            magic_link_pwd_first: l.magic_link_pwd_first,
        }
    }

    fn defaults() -> Self {
        Self {
            refresh_token_grace_time: Duration::from_secs(5),
            refresh_token_lifetime: Duration::from_secs(48 * 3600),
            session_lifetime: Duration::from_secs(10 * 3600),
            session_timeout: Duration::from_secs(90 * 60),
            session_renew_mfa: false,
            magic_link_pwd_reset: Duration::from_secs(30 * 60),
            magic_link_pwd_first: Duration::from_secs(3 * 24 * 3600),
        }
    }

    fn to_api(self) -> LifetimesSettings {
        LifetimesSettings {
            refresh_token_grace_time: self.refresh_token_grace_time.as_secs(),
            refresh_token_lifetime: self.refresh_token_lifetime.as_secs(),
            session_lifetime: self.session_lifetime.as_secs(),
            session_timeout: self.session_timeout.as_secs(),
            session_renew_mfa: self.session_renew_mfa,
            magic_link_pwd_reset: self.magic_link_pwd_reset.as_secs(),
            magic_link_pwd_first: self.magic_link_pwd_first.as_secs(),
        }
    }

    fn from_api(v: &LifetimesSettings) -> Self {
        Self {
            refresh_token_grace_time: Duration::from_secs(v.refresh_token_grace_time),
            refresh_token_lifetime: Duration::from_secs(v.refresh_token_lifetime),
            session_lifetime: Duration::from_secs(v.session_lifetime),
            session_timeout: Duration::from_secs(v.session_timeout),
            session_renew_mfa: v.session_renew_mfa,
            magic_link_pwd_reset: Duration::from_secs(v.magic_link_pwd_reset),
            magic_link_pwd_first: Duration::from_secs(v.magic_link_pwd_first),
        }
    }
}

/// Session, token and link lifetimes right now: the DB row if there is one, the config file
/// otherwise.
pub fn lifetimes() -> Lifetimes {
    live().lifetimes.unwrap_or_else(Lifetimes::from_file)
}

pub fn lifetimes_view() -> LifetimesSettingsResponse {
    LifetimesSettingsResponse {
        values: lifetimes().to_api(),
        from_db: live().lifetimes.is_some(),
    }
}

pub async fn save_lifetimes(v: LifetimesSettings) -> Result<(), ErrorResponse> {
    const MIN: u64 = 60;
    const DAY: u64 = 24 * 3600;
    if v.refresh_token_grace_time > 3600 {
        return Err(bad("The refresh token grace time can be at most 1 hour"));
    }
    if !(MIN..=365 * DAY).contains(&v.refresh_token_lifetime) {
        return Err(bad(
            "The refresh token lifetime must be between 1 minute and 365 days",
        ));
    }
    if !(5 * MIN..=90 * DAY).contains(&v.session_lifetime) {
        return Err(bad(
            "The session lifetime must be between 5 minutes and 90 days",
        ));
    }
    if !(MIN..=90 * DAY).contains(&v.session_timeout) {
        return Err(bad(
            "The session timeout must be between 1 minute and 90 days",
        ));
    }
    if v.session_timeout > v.session_lifetime {
        return Err(bad(
            "The session timeout cannot be longer than the session lifetime",
        ));
    }
    for (name, secs) in [
        ("password reset link", v.magic_link_pwd_reset),
        ("first password link", v.magic_link_pwd_first),
    ] {
        if !(MIN..=30 * DAY).contains(&secs) {
            return Err(bad(format!(
                "The {name} lifetime must be between 1 minute and 30 days"
            )));
        }
    }

    upsert(KEY_LIFETIMES, &serde_json::to_string(&v)?).await?;
    reload().await
}

/// The `general` group: values that apply everywhere
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GeneralStored {
    pub app_name: String,
}

fn key_channel(channel: &MessageChannel) -> String {
    format!("channel.{channel}")
}

#[derive(Debug, Clone, Deserialize, FromRow, FromPgRow)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    pub updated: i64,
}

// ---- stored (DB) shapes ----------------------------------------------------------------

/// The `smtp` group as it is stored. `password` is a base64 encoded, encrypted value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SmtpStored {
    pub url: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: String,
    /// `tls`, `starttls` or `danger-insecure`
    pub tls_mode: String,
}

/// A `channel.*` group as it is stored. Secrets are base64 encoded and encrypted.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChannelStored {
    /// `""` (disabled), `smtp`, `webhook` or `http`
    pub driver: String,
    /// `webhook`
    pub url: Option<String>,
    /// `webhook`: the signing secret
    pub api_key: Option<String>,
    /// Shown in the UI. Empty: the name.
    #[serde(default)]
    pub label: String,
    /// `email`, `phone` or `attr:<name>`. Empty: `email` for the channel `email`, else `phone`.
    #[serde(default)]
    pub recipient: String,
    /// Unix timestamp after which the channel stops working
    #[serde(default)]
    pub expires: Option<i64>,
    /// `smtp` of a custom channel: its own server
    #[serde(default)]
    pub smtp: Option<SmtpStored>,
    #[serde(default)]
    pub http: Option<HttpStored>,
}

/// An `http` channel as stored
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HttpStored {
    pub method: String,
    pub url: String,
    pub headers: Vec<HttpKvStored>,
    /// `json` or `form`
    pub body_format: String,
    pub body: Vec<HttpKvStored>,
}

/// `value` of a secret is encrypted
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HttpKvStored {
    pub name: String,
    pub value: String,
    pub secret: bool,
}

// ---- effective (runtime) shapes --------------------------------------------------------

/// The SMTP settings in use: the DB row if there is one, the config file otherwise.
#[derive(Debug, Clone)]
pub struct EffectiveSmtp {
    pub url: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: String,
    pub tls_mode: EmailTlsMode,
    pub from_db: bool,
    /// changes with every settings reload, lets the mailer notice and reconnect
    pub generation: u64,
    /// the own server of a custom channel, never uses the XOAUTH2 of the default mailer
    pub custom: bool,
}

impl EffectiveSmtp {
    /// `true` if there is a server to talk to
    pub fn is_configured(&self) -> bool {
        self.url.as_deref().is_some_and(|u| !u.trim().is_empty())
    }
}

/// The `user_values` group: which profile fields are shown / required. `preferred_username`
/// has a compiled regex and stays in the config file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserValuesStored {
    pub given_name: String,
    pub family_name: String,
    pub birthdate: String,
    pub gender: String,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub country: String,
    pub phone: String,
    pub tz: String,
    pub revalidate_during_login: bool,
    pub name_order_reverse: bool,
}

/// The same values with typed requirements, the way the code reads them
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UserValuesEffective {
    pub given_name: UserValueConfigValue,
    pub family_name: UserValueConfigValue,
    pub birthdate: UserValueConfigValue,
    pub gender: UserValueConfigValue,
    pub street: UserValueConfigValue,
    pub zip: UserValueConfigValue,
    pub city: UserValueConfigValue,
    pub country: UserValueConfigValue,
    pub phone: UserValueConfigValue,
    pub tz: UserValueConfigValue,
    pub revalidate_during_login: bool,
    pub name_order_reverse: bool,
}

impl UserValuesEffective {
    /// The built-in defaults, to tell whether the config file changed anything
    fn defaults() -> Self {
        use UserValueConfigValue::*;
        Self {
            given_name: Required,
            family_name: Optional,
            birthdate: Optional,
            gender: Optional,
            street: Optional,
            zip: Optional,
            city: Optional,
            country: Optional,
            phone: Optional,
            tz: Optional,
            revalidate_during_login: false,
            name_order_reverse: false,
        }
    }
}

fn uv_to_str(v: UserValueConfigValue) -> &'static str {
    match v {
        UserValueConfigValue::Required => "required",
        UserValueConfigValue::Optional => "optional",
        UserValueConfigValue::Hidden => "hidden",
    }
}

fn uv_from_str(s: &str) -> Option<UserValueConfigValue> {
    match s {
        "required" => Some(UserValueConfigValue::Required),
        "optional" => Some(UserValueConfigValue::Optional),
        "hidden" => Some(UserValueConfigValue::Hidden),
        _ => None,
    }
}

impl UserValuesEffective {
    fn from_file() -> Self {
        let v = &RauthyConfig::get().vars.user_values;
        Self {
            given_name: v.given_name,
            family_name: v.family_name,
            birthdate: v.birthdate,
            gender: v.gender,
            street: v.street,
            zip: v.zip,
            city: v.city,
            country: v.country,
            phone: v.phone,
            tz: v.tz,
            revalidate_during_login: v.revalidate_during_login,
            name_order_reverse: v.name_order_reverse,
        }
    }

    fn to_stored(self) -> UserValuesStored {
        UserValuesStored {
            given_name: uv_to_str(self.given_name).into(),
            family_name: uv_to_str(self.family_name).into(),
            birthdate: uv_to_str(self.birthdate).into(),
            gender: uv_to_str(self.gender).into(),
            street: uv_to_str(self.street).into(),
            zip: uv_to_str(self.zip).into(),
            city: uv_to_str(self.city).into(),
            country: uv_to_str(self.country).into(),
            phone: uv_to_str(self.phone).into(),
            tz: uv_to_str(self.tz).into(),
            revalidate_during_login: self.revalidate_during_login,
            name_order_reverse: self.name_order_reverse,
        }
    }
}

impl UserValuesStored {
    /// `None` if a requirement is not one of required / optional / hidden
    fn to_effective(&self) -> Option<UserValuesEffective> {
        Some(UserValuesEffective {
            given_name: uv_from_str(&self.given_name)?,
            family_name: uv_from_str(&self.family_name)?,
            birthdate: uv_from_str(&self.birthdate)?,
            gender: uv_from_str(&self.gender)?,
            street: uv_from_str(&self.street)?,
            zip: uv_from_str(&self.zip)?,
            city: uv_from_str(&self.city)?,
            country: uv_from_str(&self.country)?,
            phone: uv_from_str(&self.phone)?,
            tz: uv_from_str(&self.tz)?,
            revalidate_during_login: self.revalidate_during_login,
            name_order_reverse: self.name_order_reverse,
        })
    }
}

#[derive(Debug, Default)]
struct Live {
    lifetimes: Option<Lifetimes>,
    user_values: Option<UserValuesEffective>,
    general: Option<GeneralStored>,
    smtp: Option<(SmtpStored, Option<String>)>,
    /// ready to use: secrets decrypted once here, not per message
    channels: HashMap<String, ChannelConfig>,
    stamps: HashMap<String, i64>,
    generation: u64,
}

static LIVE: LazyLock<RwLock<Arc<Live>>> = LazyLock::new(|| RwLock::new(Arc::new(Live::default())));

fn live() -> Arc<Live> {
    LIVE.read().expect("settings lock poisoned").clone()
}

fn tls_from_str(s: &str) -> EmailTlsMode {
    match s.to_ascii_lowercase().as_str() {
        "starttls" => EmailTlsMode::StartTls,
        "danger-insecure" => EmailTlsMode::DangerInsecure,
        _ => EmailTlsMode::Tls,
    }
}

pub fn tls_to_str(m: &EmailTlsMode) -> &'static str {
    match m {
        EmailTlsMode::Tls => "tls",
        EmailTlsMode::StartTls => "starttls",
        EmailTlsMode::DangerInsecure => "danger-insecure",
    }
}

fn encrypt_secret(plain: &str) -> Result<String, ErrorResponse> {
    Ok(base64_encode(
        &EncValue::encrypt(plain.as_bytes())?.into_bytes(),
    ))
}

fn decrypt_secret(b64: &str) -> Result<String, ErrorResponse> {
    let bytes = EncValue::try_from(base64_decode(b64)?)?.decrypt()?;
    Ok(String::from_utf8_lossy(bytes.as_ref()).to_string())
}

// ---- effective accessors ---------------------------------------------------------------

/// Which profile fields are shown / required right now: the DB row if there is one, the config
/// file otherwise.
pub fn user_values() -> UserValuesEffective {
    live()
        .user_values
        .unwrap_or_else(UserValuesEffective::from_file)
}

pub fn user_values_from_db() -> bool {
    live().user_values.is_some()
}

/// The whole `user_values` config as JSON for the frontend: the file config (which has the
/// `preferred_username` block) with the DB values on top.
pub fn user_values_json() -> serde_json::Value {
    let mut v = serde_json::to_value(&RauthyConfig::get().vars.user_values).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        let stored = user_values().to_stored();
        if let Ok(serde_json::Value::Object(over)) = serde_json::to_value(stored) {
            o.extend(over);
        }
    }
    v
}

/// The name of this service as shown to users: the DB value if set, `server.app_name` otherwise.
pub fn app_name() -> String {
    live()
        .general
        .as_ref()
        .map(|g| g.app_name.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| RauthyConfig::get().vars.server.app_name.to_string())
}

/// The SMTP settings to use right now.
pub fn smtp() -> EffectiveSmtp {
    let live = live();
    if let Some((s, password)) = &live.smtp {
        return EffectiveSmtp {
            url: s.url.clone().filter(|u| !u.trim().is_empty()),
            port: s.port,
            username: s.username.clone().filter(|u| !u.trim().is_empty()),
            password: password.clone(),
            from: s.from.clone(),
            tls_mode: tls_from_str(&s.tls_mode),
            from_db: true,
            generation: live.generation,
            custom: false,
        };
    }

    let v = &RauthyConfig::get().vars.email;
    EffectiveSmtp {
        url: v.smtp_url.clone(),
        port: v.smtp_port,
        username: v.smtp_username.clone(),
        password: v.smtp_password.clone(),
        from: v.smtp_from.to_string(),
        tls_mode: tls_from_str(tls_to_str(&v.smtp_tls_mode)),
        from_db: false,
        generation: live.generation,
        custom: false,
    }
}

fn default_recipient(channel: &MessageChannel) -> &'static str {
    if channel.is_email() { "email" } else { "phone" }
}

fn decrypt_opt(what: &str, v: Option<&str>) -> Option<String> {
    match v.map(decrypt_secret) {
        Some(Ok(p)) => Some(p),
        Some(Err(err)) => {
            error!("Cannot decrypt the stored {what}: {}", err.message);
            None
        }
        None => None,
    }
}

fn smtp_effective(s: &SmtpStored, password: Option<String>, generation: u64) -> EffectiveSmtp {
    EffectiveSmtp {
        url: s.url.clone().filter(|u| !u.trim().is_empty()),
        port: s.port,
        username: s.username.clone().filter(|u| !u.trim().is_empty()),
        password,
        from: s.from.clone(),
        tls_mode: tls_from_str(&s.tls_mode),
        from_db: true,
        generation,
        custom: true,
    }
}

fn body_format(s: &str) -> BodyFormat {
    if s == "form" {
        BodyFormat::Form
    } else {
        BodyFormat::Json
    }
}

/// The ready-to-use config of a stored channel, secrets decrypted
fn build_channel(name: &str, c: &ChannelStored, generation: u64) -> ChannelConfig {
    let ch: MessageChannel = name.parse().unwrap_or_default();
    let driver = match ChannelDriver::from_name(&c.driver) {
        Some(ChannelDriver::Smtp) if ch.is_builtin() && !ch.is_email() => ChannelDriver::Disabled,
        Some(d) => d,
        None => {
            warn!(
                "Channel '{name}' uses the driver '{}', which does not exist (any more). It is \
                disabled until a new driver is chosen in the admin UI.",
                c.driver
            );
            ChannelDriver::Disabled
        }
    };

    let label = if c.label.is_empty() {
        // the built-in channels have a proper name, a custom one only its id
        RauthyConfig::get()
            .vars
            .channels
            .get(&ch)
            .map(|f| f.label.clone())
            .unwrap_or_else(|| name.to_string())
    } else {
        c.label.clone()
    };

    ChannelConfig {
        driver,
        url: c.url.clone().filter(|u| !u.trim().is_empty()),
        api_key: decrypt_opt(&format!("{name} signing secret"), c.api_key.as_deref()),
        label,
        recipient: if c.recipient.is_empty() {
            default_recipient(&ch).to_string()
        } else {
            c.recipient.clone()
        },
        expires: c.expires,
        smtp: c.smtp.as_ref().map(|s| {
            Arc::new(smtp_effective(
                s,
                decrypt_opt(&format!("{name} SMTP password"), s.password.as_deref()),
                generation,
            ))
        }),
        http: c.http.as_ref().map(|h| {
            let kv = |v: &Vec<HttpKvStored>| -> Vec<HttpKv> {
                v.iter()
                    .map(|e| HttpKv {
                        name: e.name.clone(),
                        value: if e.secret {
                            decrypt_opt(&format!("{name} secret '{}'", e.name), Some(&e.value))
                                .unwrap_or_default()
                        } else {
                            e.value.clone()
                        },
                        secret: e.secret,
                    })
                    .collect()
            };
            Arc::new(HttpRuntime {
                method: h.method.clone(),
                url: h.url.clone(),
                headers: kv(&h.headers),
                body_format: body_format(&h.body_format),
                body: kv(&h.body),
            })
        }),
    }
}

/// The config of a channel to use right now. A channel that does not exist is `Disabled`.
pub fn channel(channel: &MessageChannel) -> ChannelConfig {
    if let Some(c) = live().channels.get(channel.as_str()) {
        return c.clone();
    }
    if let Some(c) = RauthyConfig::get().vars.channels.get(channel) {
        return c.clone();
    }
    ChannelConfig::missing(channel)
}

/// `true` if the channel's current config comes from the DB
pub fn channel_from_db(channel: &MessageChannel) -> bool {
    live().channels.contains_key(channel.as_str())
}

/// All channels, the built-in ones first, then the custom ones by name
pub fn all_channels() -> Vec<MessageChannel> {
    let mut custom: Vec<String> = live()
        .channels
        .keys()
        .filter(|k| !MessageChannel::BUILTIN.contains(&k.as_str()))
        .cloned()
        .collect();
    custom.sort();
    MessageChannel::BUILTIN
        .iter()
        .map(|s| s.to_string())
        .chain(custom)
        .filter_map(|n| n.parse().ok())
        .collect()
}

/// Channels that can be used for sending right now: configured and not expired
pub fn available_channels() -> Vec<(MessageChannel, ChannelConfig)> {
    all_channels()
        .into_iter()
        .map(|c| {
            let cfg = channel(&c);
            (c, cfg)
        })
        .filter(|(_, cfg)| cfg.is_usable())
        .collect()
}

// ---- views for the admin API (no secrets) ----------------------------------------------

pub fn general_from_db() -> bool {
    live().general.is_some()
}

pub fn smtp_view() -> SmtpSettingsResponse {
    let s = smtp();
    SmtpSettingsResponse {
        url: s.url,
        port: s.port,
        username: s.username,
        has_password: s.password.is_some(),
        from: s.from,
        tls_mode: tls_to_str(&s.tls_mode).to_string(),
        from_db: s.from_db,
    }
}

fn kv_view(v: &[HttpKv]) -> Vec<HttpKvResponse> {
    v.iter()
        .map(|e| HttpKvResponse {
            name: e.name.clone(),
            value: (!e.secret).then(|| e.value.clone()),
            secret: e.secret,
        })
        .collect()
}

pub fn channel_view(ch: MessageChannel) -> ChannelSettingsResponse {
    let c = channel(&ch);
    ChannelSettingsResponse {
        driver: c.driver.as_str().to_string(),
        expired: c.expired(),
        expires: c.expires,
        url: c.url.clone(),
        has_api_key: c.api_key.is_some(),
        from_db: channel_from_db(&ch),
        label: c.label.clone(),
        recipient: c.recipient.clone(),
        builtin: ch.is_builtin(),
        smtp: c.smtp.as_deref().map(|s| SmtpSettingsResponse {
            url: s.url.clone(),
            port: s.port,
            username: s.username.clone(),
            has_password: s.password.is_some(),
            from: s.from.clone(),
            tls_mode: tls_to_str(&s.tls_mode).to_string(),
            from_db: true,
        }),
        http: c.http.as_deref().map(|h| HttpSettingsResponse {
            method: h.method.clone(),
            url: h.url.clone(),
            headers: kv_view(&h.headers),
            body_format: h.body_format.as_str().to_string(),
            body: kv_view(&h.body),
        }),
        channel: ch,
    }
}

// ---- storage ---------------------------------------------------------------------------

async fn find_all() -> Result<Vec<SettingRow>, ErrorResponse> {
    let sql = "SELECT * FROM settings";
    let rows = if is_hiqlite() {
        DB::hql().query_map(sql, params!()).await?
    } else {
        DB::pg_query(sql, &[], 0).await?
    };
    Ok(rows)
}

pub(crate) async fn upsert_raw(key: &str, value: &str) -> Result<(), ErrorResponse> {
    upsert(key, value).await
}

pub(crate) async fn delete_raw(key: &str) -> Result<(), ErrorResponse> {
    let sql = "DELETE FROM settings WHERE key = $1";
    if is_hiqlite() {
        DB::hql().execute(sql, params!(key)).await?;
    } else {
        DB::pg_execute(sql, &[&key]).await?;
    }
    Ok(())
}

async fn upsert(key: &str, value: &str) -> Result<(), ErrorResponse> {
    let now = Utc::now().timestamp();
    let sql = r#"
INSERT INTO settings (key, value, updated) VALUES ($1, $2, $3)
ON CONFLICT (key) DO UPDATE SET value = $2, updated = $3"#;
    if is_hiqlite() {
        DB::hql().execute(sql, params!(key, value, now)).await?;
    } else {
        DB::pg_execute(sql, &[&key, &value, &now]).await?;
    }
    Ok(())
}

/// Rereads all settings from the DB.
pub async fn reload() -> Result<(), ErrorResponse> {
    let rows = find_all().await?;

    let generation = LIVE.read().expect("settings lock poisoned").generation + 1;
    let mut new = Live::default();
    let mut template_rows = Vec::new();
    for row in rows {
        new.stamps.insert(row.key.clone(), row.updated);
        if row.key.starts_with("template.") {
            template_rows.push((row.key.clone(), row.value.clone()));
            continue;
        }

        if row.key == KEY_LIFETIMES {
            match serde_json::from_str::<LifetimesSettings>(&row.value) {
                Ok(l) => new.lifetimes = Some(Lifetimes::from_api(&l)),
                Err(err) => error!(?err, "Invalid `lifetimes` settings in the DB"),
            }
        } else if row.key == KEY_USER_VALUES {
            match serde_json::from_str::<UserValuesStored>(&row.value)
                .map_err(|e| e.to_string())
                .and_then(|s| {
                    s.to_effective()
                        .ok_or_else(|| "invalid requirement".to_string())
                }) {
                Ok(e) => new.user_values = Some(e),
                Err(err) => error!(%err, "Invalid `user_values` settings in the DB"),
            }
        } else if row.key == KEY_GENERAL {
            match serde_json::from_str::<GeneralStored>(&row.value) {
                Ok(g) => new.general = Some(g),
                Err(err) => error!(?err, "Invalid `general` settings in the DB"),
            }
        } else if row.key == KEY_SMTP {
            match serde_json::from_str::<SmtpStored>(&row.value) {
                Ok(s) => {
                    let password = match s.password.as_deref().map(decrypt_secret) {
                        Some(Ok(p)) => Some(p),
                        Some(Err(err)) => {
                            error!("Cannot decrypt the stored SMTP password: {}", err.message);
                            None
                        }
                        None => None,
                    };
                    new.smtp = Some((s, password));
                }
                Err(err) => error!(?err, "Invalid `smtp` settings in the DB"),
            }
        } else if let Some(name) = row.key.strip_prefix("channel.") {
            if name.parse::<MessageChannel>().is_err() {
                continue;
            }
            match serde_json::from_str::<ChannelStored>(&row.value) {
                Ok(c) => {
                    new.channels
                        .insert(name.to_string(), build_channel(name, &c, generation));
                }
                Err(err) => error!(?err, "Invalid `{}` settings in the DB", row.key),
            }
        }
    }

    {
        let mut lock = LIVE.write().expect("settings lock poisoned");
        new.generation = generation;
        *lock = Arc::new(new);
    }
    // the wording of the messages follows the same swap
    crate::message_templates::apply(&template_rows);
    Ok(())
}

/// Loads the settings and seeds groups that only exist in the config file. Call once on
/// startup, after the DB migrations.
pub async fn init() -> Result<(), ErrorResponse> {
    reload().await?;

    let live = live();
    let mut seeded = Vec::new();

    if live.general.is_none() {
        let name = RauthyConfig::get().vars.server.app_name.to_string();
        if name != "Rauthy" {
            let stored = GeneralStored { app_name: name };
            upsert(KEY_GENERAL, &serde_json::to_string(&stored)?).await?;
            seeded.push("general");
        }
    }

    if live.lifetimes.is_none() {
        let from_file = Lifetimes::from_file();
        if from_file != Lifetimes::defaults() {
            upsert(KEY_LIFETIMES, &serde_json::to_string(&from_file.to_api())?).await?;
            seeded.push("lifetimes");
        }
    }

    if live.user_values.is_none() {
        let from_file = UserValuesEffective::from_file();
        if from_file != UserValuesEffective::defaults() {
            upsert(
                KEY_USER_VALUES,
                &serde_json::to_string(&from_file.to_stored())?,
            )
            .await?;
            seeded.push("user_values");
        }
    }

    if live.smtp.is_none() {
        let v = &RauthyConfig::get().vars.email;
        if v.smtp_url.is_some() {
            let stored = SmtpStored {
                url: v.smtp_url.clone(),
                port: v.smtp_port,
                username: v.smtp_username.clone(),
                password: v.smtp_password.as_deref().map(encrypt_secret).transpose()?,
                from: v.smtp_from.to_string(),
                tls_mode: tls_to_str(&v.smtp_tls_mode).to_string(),
            };
            upsert(KEY_SMTP, &serde_json::to_string(&stored)?).await?;
            seeded.push("smtp");
        }
    }

    for name in MessageChannel::BUILTIN {
        let ch: MessageChannel = name.parse().expect("built-in channel name");
        let Some(from_file) = RauthyConfig::get().vars.channels.get(&ch) else {
            continue;
        };
        let is_default = from_file.driver
            == if ch.is_email() {
                ChannelDriver::Smtp
            } else {
                ChannelDriver::Disabled
            };
        if !live.channels.contains_key(name) && !is_default {
            upsert(
                &key_channel(&ch),
                &serde_json::to_string(&stored_from_file(from_file)?)?,
            )
            .await?;
            seeded.push(name);
        }
    }

    if !seeded.is_empty() {
        info!(
            "Settings seeded from the config file into the database: {}. From now on they are \
            managed in the admin UI (Config); the matching keys can be removed from the config \
            file / env.",
            seeded.join(", ")
        );
        reload().await?;
    }
    Ok(())
}

/// Background task: picks up changes made on other nodes.
pub async fn watch() {
    loop {
        tokio::time::sleep(WATCH_INTERVAL).await;

        let rows = match find_all().await {
            Ok(r) => r,
            Err(err) => {
                warn!("Cannot check for changed settings: {}", err.message);
                continue;
            }
        };
        let current = live();
        let same = rows.len() == current.stamps.len()
            && rows
                .iter()
                .all(|r| current.stamps.get(&r.key) == Some(&r.updated));
        if !same && let Err(err) = reload().await {
            error!("Cannot reload changed settings: {}", err.message);
        }
    }
}

// ---- updates (admin API) ---------------------------------------------------------------

fn bad(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(ErrorResponseType::BadRequest, msg.into())
}

fn valid_url(u: &str) -> bool {
    (u.starts_with("http://") || u.starts_with("https://")) && !u.contains(char::is_whitespace)
}

/// Checks the SMTP settings of a request and turns them into what is stored. `old_password` is
/// kept if the request has none.
fn smtp_stored_from(
    update: SmtpSettingsRequest,
    old_password: Option<&str>,
) -> Result<SmtpStored, ErrorResponse> {
    let url = update
        .url
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty());
    if let Some(u) = &url
        && (u.contains(char::is_whitespace) || u.contains("://"))
    {
        return Err(bad(
            "The server is a host name like smtp.example.com, without a scheme",
        ));
    }
    if update.from.parse::<lettre::message::Mailbox>().is_err() {
        return Err(bad(
            "The sender must look like `Name <address@example.com>`",
        ));
    }
    if !matches!(
        update.tls_mode.as_str(),
        "tls" | "starttls" | "danger-insecure"
    ) {
        return Err(bad(
            "tls_mode must be one of: tls, starttls, danger-insecure",
        ));
    }

    let password = match update.password.as_deref() {
        None => old_password.map(encrypt_secret).transpose()?,
        Some("") => None,
        Some(p) => Some(encrypt_secret(p)?),
    };

    Ok(SmtpStored {
        url,
        port: update.port,
        username: update
            .username
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty()),
        password,
        from: update.from.trim().to_string(),
        tls_mode: update.tls_mode,
    })
}

pub async fn save_smtp(update: SmtpSettingsRequest) -> Result<(), ErrorResponse> {
    let old = smtp();
    let stored = smtp_stored_from(update, old.password.as_deref())?;
    upsert(KEY_SMTP, &serde_json::to_string(&stored)?).await?;
    reload().await
}

pub async fn save_user_values(stored: UserValuesStored) -> Result<(), ErrorResponse> {
    let effective = stored
        .to_effective()
        .ok_or_else(|| bad("Every field must be one of: required, optional, hidden"))?;
    // the name is how a user is greeted everywhere, it cannot be hidden
    if effective.given_name == UserValueConfigValue::Hidden {
        return Err(bad("The given name cannot be hidden"));
    }
    upsert(
        KEY_USER_VALUES,
        &serde_json::to_string(&effective.to_stored())?,
    )
    .await?;
    reload().await?;
    // the config is part of the cached pages
    DB::hql().clear_cache(crate::database::Cache::Html).await?;
    Ok(())
}

pub fn user_values_view() -> rauthy_api_types::settings::UserValuesSettingsResponse {
    let s = user_values().to_stored();
    rauthy_api_types::settings::UserValuesSettingsResponse {
        values: rauthy_api_types::settings::UserValuesSettings {
            given_name: s.given_name,
            family_name: s.family_name,
            birthdate: s.birthdate,
            gender: s.gender,
            street: s.street,
            zip: s.zip,
            city: s.city,
            country: s.country,
            phone: s.phone,
            tz: s.tz,
            revalidate_during_login: s.revalidate_during_login,
            name_order_reverse: s.name_order_reverse,
        },
        from_db: user_values_from_db(),
    }
}

pub async fn save_user_values_request(
    v: rauthy_api_types::settings::UserValuesSettings,
) -> Result<(), ErrorResponse> {
    save_user_values(UserValuesStored {
        given_name: v.given_name,
        family_name: v.family_name,
        birthdate: v.birthdate,
        gender: v.gender,
        street: v.street,
        zip: v.zip,
        city: v.city,
        country: v.country,
        phone: v.phone,
        tz: v.tz,
        revalidate_during_login: v.revalidate_during_login,
        name_order_reverse: v.name_order_reverse,
    })
    .await
}

pub async fn save_general(app_name: &str) -> Result<(), ErrorResponse> {
    let name = app_name.trim();
    if name.is_empty() || name.chars().count() > 48 {
        return Err(bad("The name must have 1 to 48 characters"));
    }
    if name.contains(['<', '>', '"', '\\', '\n', '\r']) {
        return Err(bad("The name must not contain < > \" \\ or line breaks"));
    }
    let stored = GeneralStored {
        app_name: name.to_string(),
    };
    upsert(KEY_GENERAL, &serde_json::to_string(&stored)?).await?;
    reload().await?;
    // pages carry the name, drop the cached ones
    DB::hql().clear_cache(crate::database::Cache::Html).await?;
    Ok(())
}

const HTTP_METHODS: [&str; 3] = ["POST", "PUT", "PATCH"];
/// headers Rauthy or the HTTP client set themselves
const FORBIDDEN_HEADERS: [&str; 5] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "upgrade",
];

fn valid_header_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 64
        && n.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

fn kv_stored(
    section: &str,
    req: Vec<HttpKvRequest>,
    old: &[HttpKv],
    header: bool,
) -> Result<Vec<HttpKvStored>, ErrorResponse> {
    if req.len() > 30 {
        return Err(bad(format!("At most 30 {section} entries")));
    }
    let mut out: Vec<HttpKvStored> = Vec::with_capacity(req.len());
    for kv in req {
        let name = kv.name.trim().to_string();
        if header {
            if !valid_header_name(&name) {
                return Err(bad(format!("'{name}' is not a valid header name")));
            }
            if FORBIDDEN_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
                return Err(bad(format!("The header '{name}' is set by Rauthy")));
            }
        } else if name.is_empty()
            || name.len() > 64
            || name.contains(char::is_whitespace)
            || name.starts_with('.')
            || name.ends_with('.')
            || name.contains("..")
        {
            return Err(bad(format!("'{name}' is not a valid body attribute name")));
        }
        if out.iter().any(|o| o.name.eq_ignore_ascii_case(&name)) {
            return Err(bad(format!("'{name}' is in the {section} twice")));
        }

        let value = match (kv.secret, kv.value) {
            // a secret that is not sent again stays as it is
            (true, None) => {
                let old_v = old
                    .iter()
                    .find(|o| o.secret && o.name.eq_ignore_ascii_case(&name))
                    .ok_or_else(|| bad(format!("The secret '{name}' needs a value")))?;
                encrypt_secret(&old_v.value)?
            }
            (true, Some(v)) => encrypt_secret(&v)?,
            (false, v) => v.unwrap_or_default(),
        };
        if value.len() > 4000 {
            return Err(bad(format!("The value of '{name}' is too long")));
        }
        out.push(HttpKvStored {
            name,
            value,
            secret: kv.secret,
        });
    }
    Ok(out)
}

/// Checks an `http` channel and turns it into what is stored. Secrets that are not sent again
/// are taken over from `old`.
fn http_stored_from(
    req: HttpSettingsRequest,
    old: Option<&HttpRuntime>,
) -> Result<HttpStored, ErrorResponse> {
    let method = req.method.trim().to_ascii_uppercase();
    if !HTTP_METHODS.contains(&method.as_str()) {
        return Err(bad("method must be one of: POST, PUT, PATCH"));
    }
    let url = req.url.trim().to_string();
    if !valid_url(&url) {
        return Err(bad("A valid http(s) URL is required"));
    }
    if !matches!(req.body_format.as_str(), "json" | "form") {
        return Err(bad("body_format must be json or form"));
    }

    let no_kv: Vec<HttpKv> = Vec::new();
    let headers = kv_stored(
        "header",
        req.headers,
        old.map(|o| &o.headers).unwrap_or(&no_kv),
        true,
    )?;
    let body = kv_stored(
        "body",
        req.body,
        old.map(|o| &o.body).unwrap_or(&no_kv),
        false,
    )?;

    // without the recipient and the text, nothing useful can be sent. Secrets do not count, they
    // are used as they are.
    let visible = |p: &str| {
        url.contains(p)
            || headers
                .iter()
                .chain(body.iter())
                .any(|e| !e.secret && e.value.contains(p))
    };
    for needed in ["{{to}}", "{{text}}"] {
        if !visible(needed) {
            return Err(bad(format!(
                "Nothing in the request uses {needed}: the provider would never learn the {}",
                if needed == "{{to}}" {
                    "recipient"
                } else {
                    "message"
                }
            )));
        }
    }

    Ok(HttpStored {
        method,
        url,
        headers,
        body_format: req.body_format,
        body,
    })
}

/// Creates or changes a channel. A new name creates a custom channel.
pub async fn save_channel(
    ch: MessageChannel,
    update: ChannelSettingsRequest,
) -> Result<(), ErrorResponse> {
    let driver = ChannelDriver::from_name(&update.driver)
        .ok_or_else(|| bad("driver must be one of: smtp, webhook, http, or empty to disable"))?;
    let exists = channel_from_db(&ch) || ch.is_builtin();
    if driver == ChannelDriver::Disabled && !exists {
        return Err(bad("A new channel needs a driver"));
    }
    if driver == ChannelDriver::Smtp && ch.as_str() == "sms" {
        return Err(bad("SMS cannot be sent through SMTP"));
    }
    let old = channel(&ch);

    // the built-in channels use the default email and phone number and never expire
    let recipient = if ch.is_builtin() {
        default_recipient(&ch).to_string()
    } else {
        match update.recipient.map(|r| r.trim().to_string()) {
            Some(r) if !r.is_empty() => r,
            _ => old.recipient.clone(),
        }
    };
    if !crate::channels::recipient_source_is_valid(&recipient) {
        return Err(bad("recipient must be `email`, `phone` or `attr:<name>`"));
    }
    if let Some(attr) = recipient.strip_prefix("attr:")
        && !UserAttrConfigEntity::find_all_as_set()
            .await?
            .contains(attr)
    {
        return Err(bad(format!("The attribute '{attr}' does not exist")));
    }
    if ch.is_builtin() && update.expires.is_some() {
        return Err(bad("The email and sms channels cannot expire"));
    }

    let label = update
        .label
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| old.label.clone());
    if label.chars().count() > 64 || label.contains(['<', '>', '"', '\\', '\n', '\r']) {
        return Err(bad(
            "The label must be at most 64 characters without < > \" \\ or line breaks",
        ));
    }

    let mut stored = ChannelStored {
        driver: driver.as_str().to_string(),
        label,
        recipient,
        expires: update.expires,
        ..Default::default()
    };

    match driver {
        ChannelDriver::Disabled => {}
        ChannelDriver::Smtp => {
            // `email` uses the server under Email (SMTP), a custom channel has its own
            if !ch.is_email() {
                let req = update
                    .smtp
                    .ok_or_else(|| bad("The SMTP server of this channel is missing"))?;
                let old_pw = old.smtp.as_ref().and_then(|s| s.password.clone());
                let smtp = smtp_stored_from(req, old_pw.as_deref())?;
                if smtp.url.is_none() {
                    return Err(bad("The SMTP server is missing"));
                }
                stored.smtp = Some(smtp);
            }
        }
        ChannelDriver::Webhook => {
            let url = update
                .url
                .map(|u| u.trim().trim_end_matches('/').to_string())
                .filter(|u| !u.is_empty());
            match &url {
                Some(u) if valid_url(u) => {}
                _ => return Err(bad("A valid http(s) URL is required for this driver")),
            }
            stored.url = url;
            stored.api_key = match update.api_key.as_deref() {
                None => old.api_key.as_deref().map(encrypt_secret).transpose()?,
                Some("") => None,
                Some(k) => Some(encrypt_secret(k)?),
            };
        }
        ChannelDriver::Http => {
            let req = update
                .http
                .ok_or_else(|| bad("The request of this http channel is missing"))?;
            stored.http = Some(http_stored_from(req, old.http.as_deref())?);
        }
    }

    upsert(&key_channel(&ch), &serde_json::to_string(&stored)?).await?;
    reload().await
}

/// Removes an expired custom channel and every link of an attribute to it. A channel that works
/// has to be given an expiry date in the past first, so users have been told. The built-in
/// channels can only be disabled.
pub async fn delete_channel(ch: &MessageChannel) -> Result<(), ErrorResponse> {
    if ch.is_builtin() {
        return Err(bad(
            "The email and sms channels cannot be deleted, disable them instead",
        ));
    }
    if !channel_from_db(ch) {
        return Err(ErrorResponse::new(
            ErrorResponseType::NotFound,
            "Unknown channel",
        ));
    }
    if !channel(ch).expired() {
        return Err(bad(
            "Only an expired channel can be deleted. Set its expiry date to now or the past first.",
        ));
    }
    crate::entity::attr_verify_channels::AttrVerifyChannels::remove_channel(ch.as_str()).await?;

    let sql = "DELETE FROM settings WHERE key = $1";
    let key = key_channel(ch);
    if is_hiqlite() {
        DB::hql().execute(sql, params!(key)).await?;
    } else {
        DB::pg_execute(sql, &[&key]).await?;
    }
    reload().await
}

/// A built-in channel as the config file describes it, ready to store
fn stored_from_file(f: &ChannelConfig) -> Result<ChannelStored, ErrorResponse> {
    Ok(ChannelStored {
        driver: f.driver.as_str().to_string(),
        url: f.url.clone(),
        api_key: f.api_key.as_deref().map(encrypt_secret).transpose()?,
        ..Default::default()
    })
}

/// Copies the current config file / env values of a group into the DB, replacing what is there.
pub async fn import_from_file(group: &str) -> Result<(), ErrorResponse> {
    if group == KEY_LIFETIMES {
        upsert(
            KEY_LIFETIMES,
            &serde_json::to_string(&Lifetimes::from_file().to_api())?,
        )
        .await?;
        return reload().await;
    }
    if group == KEY_USER_VALUES {
        upsert(
            KEY_USER_VALUES,
            &serde_json::to_string(&UserValuesEffective::from_file().to_stored())?,
        )
        .await?;
        reload().await?;
        DB::hql().clear_cache(crate::database::Cache::Html).await?;
        return Ok(());
    }
    if group == KEY_GENERAL {
        let stored = GeneralStored {
            app_name: RauthyConfig::get().vars.server.app_name.to_string(),
        };
        upsert(KEY_GENERAL, &serde_json::to_string(&stored)?).await?;
        reload().await?;
        DB::hql().clear_cache(crate::database::Cache::Html).await?;
        return Ok(());
    }
    if group == KEY_SMTP {
        let v = &RauthyConfig::get().vars.email;
        let stored = SmtpStored {
            url: v.smtp_url.clone(),
            port: v.smtp_port,
            username: v.smtp_username.clone(),
            password: v.smtp_password.as_deref().map(encrypt_secret).transpose()?,
            from: v.smtp_from.to_string(),
            tls_mode: tls_to_str(&v.smtp_tls_mode).to_string(),
        };
        upsert(KEY_SMTP, &serde_json::to_string(&stored)?).await?;
    } else if let Some(Ok(ch)) = group
        .strip_prefix("channel.")
        .map(|n| n.parse::<MessageChannel>())
    {
        let Some(f) = RauthyConfig::get().vars.channels.get(&ch) else {
            return Err(bad("Only the built-in channels exist in the config file"));
        };
        let stored = stored_from_file(f)?;
        upsert(&key_channel(&ch), &serde_json::to_string(&stored)?).await?;
    } else {
        return Err(bad("Unknown settings group"));
    }
    reload().await
}

/// Raw rows for the DB migration tooling
pub async fn find_all_rows() -> Result<Vec<SettingRow>, ErrorResponse> {
    find_all().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_mode_roundtrip() {
        for s in ["tls", "starttls", "danger-insecure"] {
            assert_eq!(tls_to_str(&tls_from_str(s)), s);
        }
        // unknown values fall back to the safe default
        assert_eq!(tls_to_str(&tls_from_str("whatever")), "tls");
    }

    #[test]
    fn driver_names() {
        assert_eq!(ChannelDriver::from_name("SMTP"), Some(ChannelDriver::Smtp));
        assert_eq!(
            ChannelDriver::from_name("webhook"),
            Some(ChannelDriver::Webhook)
        );
        assert_eq!(ChannelDriver::from_name("http"), Some(ChannelDriver::Http));
        assert_eq!(ChannelDriver::from_name(""), Some(ChannelDriver::Disabled));
        // gone
        assert_eq!(ChannelDriver::from_name("notifyd"), None);
    }

    #[test]
    fn header_names() {
        assert!(valid_header_name("Authorization"));
        assert!(valid_header_name("X-Api-Key"));
        assert!(!valid_header_name("Bad Header"));
        assert!(!valid_header_name(""));
        assert!(!valid_header_name("a:b"));
    }

    #[test]
    fn channel_names() {
        assert!("sms".parse::<MessageChannel>().is_ok());
        assert!("whatsapp_user2".parse::<MessageChannel>().is_ok());
        assert!("".parse::<MessageChannel>().is_err());
        assert!("2fa".parse::<MessageChannel>().is_err());
        assert!("Has-Dash".parse::<MessageChannel>().is_err());
        assert!("a".repeat(33).parse::<MessageChannel>().is_err());
    }

    #[test]
    fn stored_settings_json_roundtrip() {
        let s = SmtpStored {
            url: Some("smtp.example.com".into()),
            port: Some(587),
            username: Some("u".into()),
            password: Some("b64".into()),
            from: "Rauthy <r@example.com>".into(),
            tls_mode: "starttls".into(),
        };
        let back: SmtpStored = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(s, back);
    }
}
