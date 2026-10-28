use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Info,
    Notice,
    Warning,
    Critical,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum EventType {
    InvalidLogins,
    IpBlacklisted,
    IpBlacklistRemoved,
    JwksRotated,
    NewUserRegistered,
    NewRauthyAdmin,
    NewRauthyVersion,
    PossibleBruteForce,
    RauthyStarted,
    RauthyHealthy,
    RauthyUnhealthy,
    SecretsMigrated,
    UserEmailChange,
    UserPasswordReset,
    Test,
    BackchannelLogoutFailed,
    ScimTaskFailed,
    ForcedLogout,
    UserLoginRevoke,
    SuspiciousApiScan,
    LoginNewLocation,
    TokenIssued,
    CredentialStuffing,
    EmailSendError,
}

#[derive(Deserialize, Validate, ToSchema, IntoParams)]
pub struct EventsListenParams {
    /// Validation: `0 <= latest <= 1000`
    #[validate(range(min = 0, max = 1000))]
    pub latest: Option<u16>,
    pub level: Option<EventLevel>,
    /// Also send the read marker of the logged-in user as the named SSE event `read` (once when
    /// the stream opens, and again whenever it moves). Only for sessions, not for API keys.
    pub read: Option<bool>,
}

#[derive(Deserialize, Validate, ToSchema, IntoParams)]
pub struct AuditListParams {
    /// Only the entries about this user, client, ...
    #[validate(length(max = 256))]
    pub target_id: Option<String>,
    /// Only entries older than this Unix timestamp in milliseconds, to page back. Default: now.
    #[validate(range(min = 0))]
    pub before: Option<i64>,
    /// Validation: `1 <= limit <= 200`, default 50
    #[validate(range(min = 1, max = 200))]
    pub limit: Option<u16>,
}

#[derive(Serialize, ToSchema)]
pub struct AuditEntryResponse {
    pub id: String,
    /// Unix timestamp in milliseconds
    pub at: i64,
    /// The id of the user who did it, `system` for the background tasks, `api_key:<name>` for an
    /// API key
    pub actor: String,
    /// The email of the user, if known. Without one, show the id.
    pub actor_label: Option<String>,
    /// `create`, `update`, `delete`, `action` (a change that is none of these), or a name like
    /// `inactivity_reminder_sent` for what the background tasks do
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    /// The email of the target, if it is a user and its email is known
    pub target_label: Option<String>,
    /// For changes through the API: the method and the route, like `PUT /users/{id}`
    pub details: Option<String>,
}

#[derive(Deserialize, Validate, ToSchema, IntoParams)]
pub struct AuditSummaryParams {
    #[validate(length(min = 1, max = 256))]
    pub target_id: String,
}

/// Who created something, and who changed it last
#[derive(Serialize, ToSchema)]
pub struct AuditSummaryResponse {
    pub created_by: Option<String>,
    pub created_by_label: Option<String>,
    /// Unix timestamp in milliseconds
    pub created_at: Option<i64>,
    pub modified_by: Option<String>,
    pub modified_by_label: Option<String>,
    /// Unix timestamp in milliseconds
    pub modified_at: Option<i64>,
}

/// How far the logged-in user has read the events
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct EventsReadState {
    /// Timestamp in milliseconds, like `EventResponse::timestamp`. `0` if nothing was read yet.
    #[validate(range(min = 0))]
    pub read_until: i64,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct EventsRequest {
    /// Unix timestamp in seconds
    #[validate(range(min = 1719784800))]
    pub from: i64,
    /// Unix timestamp in seconds
    #[validate(range(min = 1719784800))]
    pub until: Option<i64>,
    pub level: EventLevel,
    pub typ: Option<EventType>,
}

#[derive(Serialize, ToSchema)]
pub struct EventResponse {
    pub id: String,
    pub timestamp: i64,
    pub level: EventLevel,
    pub typ: EventType,
    pub ip: Option<String>,
    pub data: Option<i64>,
    pub text: Option<String>,
}
