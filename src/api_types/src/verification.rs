use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactKind {
    Email,
    Phone,
}

impl ContactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
        }
    }
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
    /// `true` if an SMS channel is configured and a phone number can be verified by code
    pub phone_verify_available: bool,
}

/// Verification state of a custom attribute of type `verified_email` / `verified_phone`
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct AttrVerificationResponse {
    pub kind: ContactKind,
    /// `None` if the user has no value for the attribute yet
    pub value: Option<String>,
    pub verified: bool,
    /// `false` if the needed channel (SMS for phone numbers) is not configured
    pub verify_available: bool,
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
