use crate::ReqPrincipal;
use actix_web::web::{Json, Path, Query};
use actix_web::{HttpResponse, get, post, put};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::users::UserAttrConfigTyp;
use rauthy_api_types::verification::{
    AttrVerificationResponse, ChannelOption, ContactKind, ContactStatus,
    VerificationStatusResponse, VerifyConfirmRequest, VerifySetRequest, VerifyStartQuery,
};
use rauthy_data::channels::{self, MessagePurpose};
use rauthy_data::entity::attr_verify_channels::AttrVerifyChannels;
use rauthy_data::entity::contact_verifications::ContactVerification;
use rauthy_data::entity::user_attr::{UserAttrConfigEntity, UserAttrValueEntity};
use rauthy_data::entity::users::User;
use rauthy_data::entity::users_values::UserValues;
use rauthy_data::settings;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use validator::Validate;

fn option(ch: &MessageChannel) -> ChannelOption {
    ChannelOption {
        name: ch.as_str().to_string(),
        label: settings::channel(ch).label,
    }
}

/// The working channels a code for the user's own email address / phone number can go through:
/// the ones tied to that value. The first one is the default.
fn standard_channels(kind: ContactKind) -> Vec<MessageChannel> {
    let wanted = match kind {
        ContactKind::Email => "email",
        ContactKind::Phone => "phone",
        ContactKind::Other => return Vec::new(),
    };
    settings::available_channels()
        .into_iter()
        .filter(|(_, cfg)| cfg.recipient == wanted)
        .map(|(ch, _)| ch)
        .collect()
}

/// The channel the user asked for, or the first one
fn pick(options: &[MessageChannel], wanted: Option<&str>) -> Result<MessageChannel, ErrorResponse> {
    match wanted.filter(|w| !w.is_empty()) {
        Some(w) => options
            .iter()
            .find(|c| c.as_str() == w)
            .cloned()
            .ok_or_else(|| {
                ErrorResponse::new(
                    ErrorResponseType::BadRequest,
                    format!("The channel '{w}' cannot be used for this value"),
                )
            }),
        None => options.first().cloned().ok_or_else(|| {
            ErrorResponse::new(
                ErrorResponseType::BadRequest,
                "No channel is configured that can verify this value",
            )
        }),
    }
}

/// `(subject, body)` of the verification message, in the language of the user. The wording is
/// editable under Config -> Message templates.
fn verification_message(user: &User, code: &str) -> (String, String) {
    rauthy_data::message_templates::verification(
        user.language.as_str(),
        code,
        &user.email_recipient_name(),
    )
}

async fn phone_of(user_id: &str) -> Result<Option<String>, ErrorResponse> {
    Ok(UserValues::find(user_id)
        .await?
        .and_then(|v| v.phone)
        .filter(|p| !p.is_empty()))
}

/// The contact value of the given kind, if the user has one.
async fn value_of(user: &User, kind: ContactKind) -> Result<Option<String>, ErrorResponse> {
    match kind {
        ContactKind::Email => Ok(Some(user.email.clone())),
        ContactKind::Phone => phone_of(&user.id).await,
        ContactKind::Other => Ok(None),
    }
}

fn no_value(kind: ContactKind) -> ErrorResponse {
    ErrorResponse::new(
        ErrorResponseType::BadRequest,
        format!("The user has no {}", kind.as_str()),
    )
}

/// GET the verification state of the user's email address and phone number
///
/// **Permissions**
/// - the user itself
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/users/{id}/verification",
    tag = "users",
    responses(
        (status = 200, description = "Ok", body = VerificationStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/users/{id}/verification")]
pub async fn get_verification(
    id: Path<String>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    let id = id.into_inner();
    principal.validate_user_or_admin(&id)?;

    let user = User::find(id).await?;
    let phone_channels = standard_channels(ContactKind::Phone);
    let phone = match phone_of(&user.id).await? {
        Some(p) => {
            let verified =
                ContactVerification::is_verified(&user.id, ContactKind::Phone.as_str(), &p).await?;
            Some(ContactStatus { value: p, verified })
        }
        None => None,
    };

    Ok(HttpResponse::Ok().json(VerificationStatusResponse {
        email: ContactStatus {
            value: user.email.clone(),
            verified: user.email_verified,
        },
        phone,
        phone_verify_available: !phone_channels.is_empty(),
        email_channels: standard_channels(ContactKind::Email)
            .iter()
            .map(option)
            .collect(),
        phone_channels: phone_channels.iter().map(option).collect(),
    }))
}

/// Send a verification code to the user's email address or phone number
///
/// A code is valid for 10 minutes and allows 5 wrong guesses. A new one can be requested after
/// 60 seconds. `channel` picks the channel to send it through, the default is the first one that
/// can deliver to the value.
///
/// **Permissions**
/// - the user itself
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/users/{id}/verification/{kind}/start",
    tag = "users",
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 429, description = "TooManyRequests", body = ErrorResponse),
    ),
)]
#[post("/users/{id}/verification/{kind}/start")]
pub async fn post_verification_start(
    path: Path<(String, ContactKind)>,
    query: Query<VerifyStartQuery>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    let (id, kind) = path.into_inner();
    principal.validate_user_or_admin(&id)?;

    let user = User::find(id).await?;
    let value = value_of(&user, kind).await?.ok_or_else(|| no_value(kind))?;

    if kind == ContactKind::Email && user.email_verified {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "This value is already verified",
        ));
    }
    let channel = pick(&standard_channels(kind), query.channel.as_deref())?;

    let code = ContactVerification::start(&user.id, kind.as_str(), &value).await?;
    let (subject, body) = verification_message(&user, &code);
    channels::deliver(
        &channel,
        &user,
        value,
        MessagePurpose::Verification,
        &subject,
        body,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}

/// Confirm the code that has been sent to the user
///
/// **Permissions**
/// - the user itself
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/users/{id}/verification/{kind}/confirm",
    tag = "users",
    request_body = VerifyConfirmRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/users/{id}/verification/{kind}/confirm")]
pub async fn post_verification_confirm(
    path: Path<(String, ContactKind)>,
    principal: ReqPrincipal,
    Json(payload): Json<VerifyConfirmRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    let (id, kind) = path.into_inner();
    principal.validate_user_or_admin(&id)?;
    payload.validate()?;

    let mut user = User::find(id).await?;
    let value = value_of(&user, kind).await?.ok_or_else(|| no_value(kind))?;

    ContactVerification::confirm(&user.id, kind.as_str(), &value, &payload.code).await?;
    if kind == ContactKind::Email {
        user.email_verified = true;
        user.save(None).await?;
    }

    Ok(HttpResponse::Ok().finish())
}

/// Mark the user's email address or phone number as verified, or revoke it
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/users/{id}/verification/{kind}",
    tag = "users",
    request_body = VerifySetRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/users/{id}/verification/{kind}")]
pub async fn put_verification(
    path: Path<(String, ContactKind)>,
    principal: ReqPrincipal,
    Json(payload): Json<VerifySetRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let (id, kind) = path.into_inner();

    let mut user = User::find(id).await?;
    let value = value_of(&user, kind).await?.ok_or_else(|| no_value(kind))?;

    ContactVerification::set_verified(&user.id, kind.as_str(), &value, payload.verified).await?;
    if kind == ContactKind::Email && user.email_verified != payload.verified {
        user.email_verified = payload.verified;
        user.save(None).await?;
    }

    Ok(HttpResponse::Ok().finish())
}

// ---- custom attributes of type `verified_email` / `verified_phone` -------------------------

/// The kind and the current value of a verifiable attribute of the user. The verification is
/// stored under the key `attr:<name>`.
async fn attr_contact(
    user_id: &str,
    name: &str,
) -> Result<(ContactKind, Option<UserAttrConfigTyp>, Option<String>), ErrorResponse> {
    let config = UserAttrConfigEntity::find(name.to_string()).await?;
    let kind = match config.typ {
        Some(UserAttrConfigTyp::VerifiedEmail) => ContactKind::Email,
        Some(UserAttrConfigTyp::VerifiedPhone) => ContactKind::Phone,
        Some(UserAttrConfigTyp::Verified) => ContactKind::Other,
        _ => {
            return Err(ErrorResponse::new(
                ErrorResponseType::BadRequest,
                "The attribute is not of a verifiable type",
            ));
        }
    };

    let value = UserAttrValueEntity::find_for_user(user_id)
        .await?
        .into_iter()
        .find(|v| v.key == name)
        .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v.value).ok())
        .and_then(|v| v.as_str().map(String::from))
        .filter(|v| !v.is_empty());

    Ok((kind, config.typ, value))
}

fn attr_key(name: &str) -> String {
    format!("attr:{name}")
}

/// GET the verification state of a verifiable custom attribute
///
/// **Permissions**
/// - the user itself
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/users/{id}/attr/{name}/verification",
    tag = "users",
    responses(
        (status = 200, description = "Ok", body = AttrVerificationResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/users/{id}/attr/{name}/verification")]
pub async fn get_attr_verification(
    path: Path<(String, String)>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    let (id, name) = path.into_inner();
    principal.validate_user_or_admin(&id)?;

    let (kind, typ, value) = attr_contact(&id, &name).await?;
    let channels = AttrVerifyChannels::usable(&name, typ).await?;
    let verified = match &value {
        Some(v) => ContactVerification::is_verified(&id, &attr_key(&name), v).await?,
        None => false,
    };

    Ok(HttpResponse::Ok().json(AttrVerificationResponse {
        kind,
        value,
        verified,
        verify_available: !channels.is_empty(),
        channels: channels.iter().map(option).collect(),
    }))
}

/// Send a verification code to the value stored in the attribute, through one of the channels
/// linked to it (`channel`, default: the first one)
///
/// **Permissions**
/// - the user itself
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/users/{id}/attr/{name}/verification/start",
    tag = "users",
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 429, description = "TooManyRequests", body = ErrorResponse),
    ),
)]
#[post("/users/{id}/attr/{name}/verification/start")]
pub async fn post_attr_verification_start(
    path: Path<(String, String)>,
    query: Query<VerifyStartQuery>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    let (id, name) = path.into_inner();
    principal.validate_user_or_admin(&id)?;

    let (_, typ, value) = attr_contact(&id, &name).await?;
    let value = value.ok_or_else(|| {
        ErrorResponse::new(ErrorResponseType::BadRequest, "The attribute has no value")
    })?;
    let channel = pick(
        &AttrVerifyChannels::usable(&name, typ).await?,
        query.channel.as_deref(),
    )?;

    let user = User::find(id).await?;
    let code = ContactVerification::start(&user.id, &attr_key(&name), &value).await?;
    let (subject, body) = verification_message(&user, &code);
    channels::deliver(
        &channel,
        &user,
        value,
        MessagePurpose::Verification,
        &subject,
        body,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}

/// Confirm the code that has been sent for a verifiable attribute
///
/// **Permissions**
/// - the user itself
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/users/{id}/attr/{name}/verification/confirm",
    tag = "users",
    request_body = VerifyConfirmRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/users/{id}/attr/{name}/verification/confirm")]
pub async fn post_attr_verification_confirm(
    path: Path<(String, String)>,
    principal: ReqPrincipal,
    Json(payload): Json<VerifyConfirmRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    let (id, name) = path.into_inner();
    principal.validate_user_or_admin(&id)?;
    payload.validate()?;

    let (_, _, value) = attr_contact(&id, &name).await?;
    let value = value.ok_or_else(|| {
        ErrorResponse::new(ErrorResponseType::BadRequest, "The attribute has no value")
    })?;

    ContactVerification::confirm(&id, &attr_key(&name), &value, &payload.code).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Mark the value of a verifiable attribute as verified, or revoke it
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/users/{id}/attr/{name}/verification",
    tag = "users",
    request_body = VerifySetRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/users/{id}/attr/{name}/verification")]
pub async fn put_attr_verification(
    path: Path<(String, String)>,
    principal: ReqPrincipal,
    Json(payload): Json<VerifySetRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let (id, name) = path.into_inner();

    let (_, _, value) = attr_contact(&id, &name).await?;
    let value = value.ok_or_else(|| {
        ErrorResponse::new(ErrorResponseType::BadRequest, "The attribute has no value")
    })?;

    ContactVerification::set_verified(&id, &attr_key(&name), &value, payload.verified).await?;
    Ok(HttpResponse::Ok().finish())
}
