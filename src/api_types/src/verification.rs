use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactKind {
    Email,
    Phone,
    /// only for custom attributes of type `verified`
    Other,
}

impl ContactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Other => "other",
        }
    }
}

/// A channel a value can be verified through
#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct ChannelOption {
    pub name: String,
    pub label: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct ContactStatus {
    pub value: String,
    pub verified: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct VerificationStatusResponse {
    pub email: ContactStatus,
    /// `None` if the user has no phone number
    pub phone: Option<ContactStatus>,
    /// `true` if a channel is configured that can verify a phone number by code
    pub phone_verify_available: bool,
    /// The channels a code for the email address can be sent through, the first is the default
    pub email_channels: Vec<ChannelOption>,
    /// The channels a code for the phone number can be sent through, the first is the default
    pub phone_channels: Vec<ChannelOption>,
}

/// Verification state of a custom attribute of type `verified_email` / `verified_phone`
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct AttrVerificationResponse {
    pub kind: ContactKind,
    /// `None` if the user has no value for the attribute yet
    pub value: Option<String>,
    pub verified: bool,
    /// `false` if no channel is linked to the attribute, or none of them is configured
    pub verify_available: bool,
    /// The channels the user can pick to receive the code, the first is the default
    pub channels: Vec<ChannelOption>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyStartQuery {
    /// The channel to send the code through, defaults to the first one available
    pub channel: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyConfirmRequest {
    /// Validation: `^[0-9]{6}$`
    #[validate(length(equal = 6))]
    pub code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifySetRequest {
    pub verified: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttrVerifyChannelsRequest {
    /// The channels, in order of preference. Empty removes all links.
    pub channels: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct AttrVerifyChannelsResponse {
    /// The linked channels, in order of preference
    pub channels: Vec<String>,
    /// All channels that exist and are able to deliver to this attribute
    pub candidates: Vec<ChannelOption>,
}
