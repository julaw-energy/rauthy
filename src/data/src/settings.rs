//! Settings that live in the database instead of the config file.
//!
//! The config file / env stay the base. On the first start with a value in the file and nothing
//! in the DB, the group is *seeded* from the file once; from then on the DB row is authoritative
//! and can be changed from the admin UI without a restart. A changed value is applied on the
//! node that saved it immediately, and on all other nodes within [`WATCH_INTERVAL`].
//!
//! Groups so far: `smtp` (the built-in mailer) and `channel.<email|sms|whatsapp>` (see
//! [`crate::channels`]). Secrets are stored encrypted and never leave the backend.

use crate::channels::{ChannelConfig, ChannelDriver};
use crate::database::DB;
use crate::rauthy_config::{EmailTlsMode, RauthyConfig, UserValueConfigValue};
use chrono::Utc;
use cryptr::EncValue;
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::settings::{
    ChannelSettingsRequest, ChannelSettingsResponse, LifetimesSettings, LifetimesSettingsResponse,
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

fn key_channel(channel: MessageChannel) -> String {
    format!("channel.{}", channel.as_str())
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

/// A `channel.*` group as it is stored. `api_key` is base64 encoded and encrypted.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChannelStored {
    /// `""` (disabled), `smtp` (email only), `notifyd` or `webhook`
    pub driver: String,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub channel_name: String,
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
    channels: HashMap<MessageChannel, (ChannelStored, Option<String>)>,
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
    }
}

/// The config of a channel to use right now.
pub fn channel(channel: MessageChannel) -> ChannelConfig {
    if let Some((c, api_key)) = live().channels.get(&channel) {
        return ChannelConfig {
            driver: driver_from_str(channel, &c.driver),
            url: c.url.clone().filter(|u| !u.trim().is_empty()),
            api_key: api_key.clone(),
            channel_name: if c.channel_name.is_empty() {
                channel.as_str().to_string()
            } else {
                c.channel_name.clone()
            },
        };
    }
    RauthyConfig::get().vars.channels.get(channel).clone()
}

/// `true` if the channel's current config comes from the DB
pub fn channel_from_db(channel: MessageChannel) -> bool {
    live().channels.contains_key(&channel)
}

/// Channels that can be used for sending right now
pub fn available_channels() -> Vec<(MessageChannel, ChannelDriver)> {
    [
        MessageChannel::Email,
        MessageChannel::Sms,
        MessageChannel::Whatsapp,
    ]
    .into_iter()
    .map(|c| (c, channel(c).driver))
    .filter(|(_, d)| *d != ChannelDriver::Disabled)
    .collect()
}

pub fn driver_from_str(channel: MessageChannel, s: &str) -> ChannelDriver {
    match (s.trim().to_ascii_lowercase().as_str(), channel) {
        ("notifyd", _) => ChannelDriver::Notifyd,
        ("webhook", _) => ChannelDriver::Webhook,
        ("smtp", MessageChannel::Email) | ("", MessageChannel::Email) => ChannelDriver::Smtp,
        _ => ChannelDriver::Disabled,
    }
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

pub fn channel_view(ch: MessageChannel) -> ChannelSettingsResponse {
    let c = channel(ch);
    ChannelSettingsResponse {
        channel: ch,
        driver: c.driver.as_str().to_string(),
        url: c.url,
        has_api_key: c.api_key.is_some(),
        channel_name: c.channel_name,
        from_db: channel_from_db(ch),
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

    let mut new = Live::default();
    for row in rows {
        new.stamps.insert(row.key.clone(), row.updated);

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
            let Ok(ch) = name.parse::<MessageChannel>() else {
                continue;
            };
            match serde_json::from_str::<ChannelStored>(&row.value) {
                Ok(c) => {
                    let api_key = match c.api_key.as_deref().map(decrypt_secret) {
                        Some(Ok(p)) => Some(p),
                        Some(Err(err)) => {
                            error!("Cannot decrypt the stored {name} api key: {}", err.message);
                            None
                        }
                        None => None,
                    };
                    new.channels.insert(ch, (c, api_key));
                }
                Err(err) => error!(?err, "Invalid `{}` settings in the DB", row.key),
            }
        }
    }

    let mut lock = LIVE.write().expect("settings lock poisoned");
    new.generation = lock.generation + 1;
    *lock = Arc::new(new);
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

    for ch in [
        MessageChannel::Email,
        MessageChannel::Sms,
        MessageChannel::Whatsapp,
    ] {
        let from_file = RauthyConfig::get().vars.channels.get(ch);
        let is_default = from_file.driver
            == if ch == MessageChannel::Email {
                ChannelDriver::Smtp
            } else {
                ChannelDriver::Disabled
            };
        if !live.channels.contains_key(&ch) && !is_default {
            let stored = ChannelStored {
                driver: from_file.driver.as_str().to_string(),
                url: from_file.url.clone(),
                api_key: from_file
                    .api_key
                    .as_deref()
                    .map(encrypt_secret)
                    .transpose()?,
                channel_name: from_file.channel_name.clone(),
            };
            upsert(&key_channel(ch), &serde_json::to_string(&stored)?).await?;
            seeded.push(ch.as_str());
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

pub async fn save_smtp(update: SmtpSettingsRequest) -> Result<(), ErrorResponse> {
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

    let old = smtp();
    let password = match update.password.as_deref() {
        None => old.password.as_deref().map(encrypt_secret).transpose()?,
        Some("") => None,
        Some(p) => Some(encrypt_secret(p)?),
    };

    let stored = SmtpStored {
        url,
        port: update.port,
        username: update
            .username
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty()),
        password,
        from: update.from.trim().to_string(),
        tls_mode: update.tls_mode,
    };
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

pub async fn save_channel(
    ch: MessageChannel,
    update: ChannelSettingsRequest,
) -> Result<(), ErrorResponse> {
    let driver = driver_from_str(ch, &update.driver);
    if !update.driver.is_empty()
        && driver == ChannelDriver::Disabled
        && !update.driver.eq_ignore_ascii_case("none")
    {
        return Err(bad(
            "driver must be one of: smtp (email only), notifyd, webhook, or empty",
        ));
    }
    let url = update
        .url
        .map(|u| u.trim().trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty());
    if matches!(driver, ChannelDriver::Notifyd | ChannelDriver::Webhook) {
        match &url {
            Some(u) if valid_url(u) => {}
            _ => return Err(bad("A valid http(s) URL is required for this driver")),
        }
    }

    let old = channel(ch);
    let api_key = match update.api_key.as_deref() {
        None => old.api_key.as_deref().map(encrypt_secret).transpose()?,
        Some("") => None,
        Some(k) => Some(encrypt_secret(k)?),
    };

    let stored = ChannelStored {
        driver: driver.as_str().to_string(),
        url,
        api_key,
        channel_name: update
            .channel_name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| ch.as_str().to_string()),
    };
    upsert(&key_channel(ch), &serde_json::to_string(&stored)?).await?;
    reload().await
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
        let f = RauthyConfig::get().vars.channels.get(ch);
        let stored = ChannelStored {
            driver: f.driver.as_str().to_string(),
            url: f.url.clone(),
            api_key: f.api_key.as_deref().map(encrypt_secret).transpose()?,
            channel_name: f.channel_name.clone(),
        };
        upsert(&key_channel(ch), &serde_json::to_string(&stored)?).await?;
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
    fn driver_parsing_per_channel() {
        use MessageChannel::*;
        assert_eq!(driver_from_str(Email, ""), ChannelDriver::Smtp);
        assert_eq!(driver_from_str(Email, "SMTP"), ChannelDriver::Smtp);
        assert_eq!(driver_from_str(Sms, ""), ChannelDriver::Disabled);
        // `smtp` is the built-in mailer, it only exists for email
        assert_eq!(driver_from_str(Sms, "smtp"), ChannelDriver::Disabled);
        assert_eq!(driver_from_str(Whatsapp, "notifyd"), ChannelDriver::Notifyd);
        assert_eq!(driver_from_str(Sms, "webhook"), ChannelDriver::Webhook);
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
