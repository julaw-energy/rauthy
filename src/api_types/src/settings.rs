use crate::email_jobs::MessageChannel;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The SMTP settings in use. The password is never returned.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct SmtpSettingsResponse {
    pub url: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub has_password: bool,
    pub from: String,
    /// `tls`, `starttls` or `danger-insecure`
    pub tls_mode: String,
    /// `true` if the values come from the database, `false` if from the config file
    pub from_db: bool,
}

/// One name / value pair of an `http` channel, for a header or a body attribute
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct HttpKvResponse {
    pub name: String,
    /// `None` for a secret, it never leaves the backend
    pub value: Option<String>,
    pub secret: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct HttpSettingsResponse {
    /// `POST`, `PUT` or `PATCH`
    pub method: String,
    pub url: String,
    pub headers: Vec<HttpKvResponse>,
    /// `json` or `form`
    pub body_format: String,
    pub body: Vec<HttpKvResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct HttpKvRequest {
    pub name: String,
    /// For a secret, `None` keeps the stored value of the entry with the same name
    pub value: Option<String>,
    /// Secrets are stored encrypted and never shown again
    pub secret: bool,
}

/// How an `http` channel calls its provider. Values can contain `{{to}}`, `{{text}}`, `{{html}}`,
/// `{{subject}}`, `{{name}}`, `{{purpose}}`, `{{channel}}`, `{{app_name}}` and
/// `{{idempotency_key}}`. Secrets are used as they are.
#[derive(Debug, Deserialize, ToSchema)]
pub struct HttpSettingsRequest {
    /// `POST`, `PUT` or `PATCH`
    pub method: String,
    pub url: String,
    pub headers: Vec<HttpKvRequest>,
    /// `json` or `form`. A body name like `messages.0.to` builds nested JSON.
    pub body_format: String,
    pub body: Vec<HttpKvRequest>,
}

/// The config of a message channel in use. Secrets are never returned.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct ChannelSettingsResponse {
    pub channel: MessageChannel,
    /// `""` (disabled), `smtp`, `webhook` or `http`
    pub driver: String,
    /// `webhook`
    pub url: Option<String>,
    /// `webhook`: a signing secret is set
    pub has_api_key: bool,
    pub from_db: bool,
    /// Shown in the UI instead of the name
    pub label: String,
    /// Which value of a user this channel delivers to: `email`, `phone` or `attr:<name>` for a
    /// custom attribute. A user without such a value is skipped / rejected.
    pub recipient: String,
    /// `email` and `sms` always exist and cannot be deleted
    pub builtin: bool,
    /// Unix timestamp after which the channel stops working. Custom channels only.
    pub expires: Option<i64>,
    pub expired: bool,
    /// `smtp` of a custom channel: its own server
    pub smtp: Option<SmtpSettingsResponse>,
    pub http: Option<HttpSettingsResponse>,
}

/// Which profile fields users see and must fill in. Every value is `required`, `optional` or
/// `hidden`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserValuesSettings {
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
    /// ask users to confirm / complete their profile again at every login to a client
    pub revalidate_during_login: bool,
    /// show the family name before the given name
    pub name_order_reverse: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct UserValuesSettingsResponse {
    #[serde(flatten)]
    pub values: UserValuesSettings,
    /// `true` if the values come from the database, `false` if from the config file
    pub from_db: bool,
}

/// How long sessions, refresh tokens and email links stay valid. All durations are in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LifetimesSettings {
    /// Grace time for a refresh token that was just used, so parallel requests do not fail
    pub refresh_token_grace_time: u64,
    pub refresh_token_lifetime: u64,
    /// Maximum age of a session, whatever the activity
    pub session_lifetime: u64,
    /// A session without activity for this long ends
    pub session_timeout: u64,
    /// Ask for MFA again when a session is renewed
    pub session_renew_mfa: bool,
    /// Link of a password reset email
    pub magic_link_pwd_reset: u64,
    /// Link of the first-password email for a new user
    pub magic_link_pwd_first: u64,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct LifetimesSettingsResponse {
    #[serde(flatten)]
    pub values: LifetimesSettings,
    /// `true` if the values come from the database, `false` if from the config file
    pub from_db: bool,
}

/// The general settings
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct GeneralSettingsResponse {
    /// The name users see instead of "Rauthy"
    pub app_name: String,
    /// `true` if the value comes from the database, `false` if from the config file
    pub from_db: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GeneralSettingsRequest {
    /// 1 to 48 characters, without `<`, `>`, `"`, `\\` and line breaks
    pub app_name: String,
}

/// Public: just the name of this service
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct AppNameResponse {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct SettingsResponse {
    pub general: GeneralSettingsResponse,
    pub user_values: UserValuesSettingsResponse,
    pub lifetimes: LifetimesSettingsResponse,
    pub smtp: SmtpSettingsResponse,
    /// `false` while no SMTP server is set up, the admin UI points to the setup page then
    pub smtp_configured: bool,
    pub channels: Vec<ChannelSettingsResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SmtpSettingsRequest {
    /// Host name like `smtp.example.com`, empty / `None` to unset
    pub url: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    /// `None` keeps the stored password, an empty string removes it
    pub password: Option<String>,
    /// Like `Rauthy <rauthy@example.com>`
    pub from: String,
    /// `tls`, `starttls` or `danger-insecure`
    pub tls_mode: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChannelSettingsRequest {
    /// `""` (disabled), `smtp`, `webhook` or `http`. `smtp` is the server under Email (SMTP) for
    /// the channel `email`, and the channel's own server for a custom channel.
    pub driver: String,
    /// `webhook`
    pub url: Option<String>,
    /// `webhook`: the signing secret. `None` keeps the stored one, an empty string removes it.
    pub api_key: Option<String>,
    /// Shown in the UI, defaults to the name
    pub label: Option<String>,
    /// `email`, `phone` or `attr:<name>`. Fixed for `email` and `sms`.
    pub recipient: Option<String>,
    /// Custom channels only. `None` means it never expires.
    pub expires: Option<i64>,
    /// `smtp` of a custom channel
    pub smtp: Option<SmtpSettingsRequest>,
    pub http: Option<HttpSettingsRequest>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct SettingsTestResponse {
    pub message: String,
}
