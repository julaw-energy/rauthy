use rauthy_common::regex::RE_GROUPS;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

const MAX_TARGET_ENTRIES: usize = 200;

/// Multi-select recipient filter. A user receives the email if it matches at least one entry of
/// `roles`, `groups` or `users` (OR) - or any user, if all three are empty - and does not match any entry of `not_roles` / `not_groups`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct EmailTargets {
    /// Role names, any user with one of these roles is included
    #[serde(default)]
    #[validate(custom(function = "validate_target_names"))]
    pub roles: Vec<String>,
    /// Group names, any member of one of these groups is included
    #[serde(default)]
    #[validate(custom(function = "validate_target_names"))]
    pub groups: Vec<String>,
    /// User IDs
    #[serde(default)]
    #[validate(custom(function = "validate_target_names"))]
    pub users: Vec<String>,
    /// Users with one of these roles are excluded
    #[serde(default)]
    #[validate(custom(function = "validate_target_names"))]
    pub not_roles: Vec<String>,
    /// Members of one of these groups are excluded
    #[serde(default)]
    #[validate(custom(function = "validate_target_names"))]
    pub not_groups: Vec<String>,
}

impl EmailTargets {
    /// `true` if there is neither an include nor an exclude entry
    pub fn is_empty(&self) -> bool {
        !self.has_includes() && self.not_roles.is_empty() && self.not_groups.is_empty()
    }

    pub fn has_includes(&self) -> bool {
        !(self.roles.is_empty() && self.groups.is_empty() && self.users.is_empty())
    }
}

fn validate_target_names(names: &[String]) -> Result<(), ValidationError> {
    if names.len() > MAX_TARGET_ENTRIES {
        return Err(ValidationError::new("too many entries"));
    }
    for n in names {
        if n.len() < 2 || n.len() > 64 || !RE_GROUPS.is_match(n) {
            return Err(ValidationError::new("invalid target name"));
        }
    }
    Ok(())
}

/// Delivery channel of a bulk message
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageChannel {
    #[default]
    Email,
    Sms,
    Whatsapp,
}

impl MessageChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Sms => "sms",
            Self::Whatsapp => "whatsapp",
        }
    }
}

impl std::str::FromStr for MessageChannel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email" => Ok(Self::Email),
            "sms" => Ok(Self::Sms),
            "whatsapp" => Ok(Self::Whatsapp),
            _ => Err(()),
        }
    }
}

/// A channel that is available for sending, see `GET /email/channels`
#[derive(Debug, Serialize, ToSchema)]
pub struct ChannelInfo {
    pub channel: MessageChannel,
    /// `smtp`, `notifyd` or `webhook`
    pub driver: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct EmailJobRequest {
    /// Delivery channel, defaults to `email`
    #[serde(default)]
    pub channel: MessageChannel,
    /// Validation: Unix TS in the future
    pub scheduled: Option<i64>,
    pub filter_type: EmailJobFilterType,
    /// Validation: `^[a-zA-Z0-9-_/,:*\s]{2,64}$`
    #[validate(regex(path = "*RE_GROUPS", code = "^[a-zA-Z0-9-_/,:*\\s]{2,64}$"))]
    pub filter_value: Option<String>,
    /// Multi-select recipients. If given and not empty, it takes precedence over `filter_type`.
    #[validate(nested)]
    pub targets: Option<EmailTargets>,
    pub content_type: EmailContentType,
    #[validate(length(max = 1024))]
    pub subject: String,
    #[validate(length(max = 32768))]
    pub body: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EmailContentType {
    Text,
    Markdown,
    HTML,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmailJobFilterType {
    None,
    InGroup,
    NotInGroup,
    HasRole,
    HasNotRole,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmailJobResponse {
    pub id: i64,
    pub channel: MessageChannel,
    pub scheduled: Option<i64>,
    pub status: EmailJobStatus,
    pub updated: i64,
    pub last_user_ts: Option<String>,
    pub filter_type: EmailJobFilterType,
    pub filter_value: Option<String>,
    pub targets: Option<EmailTargets>,
    pub content_type: EmailContentType,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub enum EmailJobStatus {
    Open = 0,
    Finished,
    Canceled,
}
