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

/// The config of a message channel in use. The API key is never returned.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct ChannelSettingsResponse {
    pub channel: MessageChannel,
    /// `""` (disabled), `smtp` (email only), `notifyd` or `webhook`
    pub driver: String,
    pub url: Option<String>,
    pub has_api_key: bool,
    pub channel_name: String,
    pub from_db: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct SettingsResponse {
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
    /// `""` (disabled), `smtp` (email only), `notifyd` or `webhook`
    pub driver: String,
    pub url: Option<String>,
    /// `None` keeps the stored key, an empty string removes it
    pub api_key: Option<String>,
    pub channel_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct SettingsTestResponse {
    pub message: String,
}
