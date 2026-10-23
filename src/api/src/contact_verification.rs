use crate::ReqPrincipal;
use actix_web::web::{Json, Path};
use actix_web::{HttpResponse, get, post, put};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::users::UserAttrConfigTyp;
use rauthy_api_types::verification::{
    AttrVerificationResponse, ContactKind, ContactStatus, VerificationStatusResponse,
    VerifyConfirmRequest, VerifySetRequest,
};
use rauthy_data::channels::{
    self, ChannelDriver, ChannelMessage, ChannelRecipient, MessagePurpose,
};
use rauthy_data::email::custom::send_custom;
use rauthy_data::entity::contact_verifications::ContactVerification;
use rauthy_data::entity::user_attr::{UserAttrConfigEntity, UserAttrValueEntity};
use rauthy_data::entity::users::User;
use rauthy_data::entity::users_values::UserValues;
use rauthy_data::rauthy_config::RauthyConfig;
use rauthy_data::settings;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use validator::Validate;

fn sms_available() -> bool {
    settings::channel(MessageChannel::Sms).driver != ChannelDriver::Disabled
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
        phone_verify_available: sms_available(),
    }))
}

/// Send a verification code to the user's email address or phone number
///
/// A code is valid for 10 minutes and allows 5 wrong guesses. A new one can be requested after
/// 60 seconds.
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
    if kind == ContactKind::Phone && !sms_available() {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "No SMS channel is configured",
        ));
    }

    let code = ContactVerification::start(&user.id, kind.as_str(), &value).await?;
    let prefix = &RauthyConfig::get().vars.email.sub_prefix;
    let text = format!("Your {prefix} verification code is {code}. It is valid for 10 minutes.");

    match kind {
        ContactKind::Email => {
            send_custom(&user, "Verification code", Some(text), None).await?;
        }
        ContactKind::Phone => {
            let msg = ChannelMessage::new(
                MessageChannel::Sms,
                MessagePurpose::Verification,
                ChannelRecipient {
                    user_id: user.id.clone(),
                    name: user.email_recipient_name(),
                    to: value,
                },
                None,
                text,
                None,
                None,
            );
            channels::send(MessageChannel::Sms, &msg).await?;
        }
    }

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
) -> Result<(ContactKind, Option<String>), ErrorResponse> {
    let config = UserAttrConfigEntity::find(name.to_string()).await?;
    let kind = match config.typ {
        Some(UserAttrConfigTyp::VerifiedEmail) => ContactKind::Email,
        Some(UserAttrConfigTyp::VerifiedPhone) => ContactKind::Phone,
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

    Ok((kind, value))
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

    let (kind, value) = attr_contact(&id, &name).await?;
    let verified = match &value {
        Some(v) => ContactVerification::is_verified(&id, &attr_key(&name), v).await?,
        None => false,
    };

    Ok(HttpResponse::Ok().json(AttrVerificationResponse {
        kind,
        value,
        verified,
        verify_available: kind == ContactKind::Email || sms_available(),
    }))
}

/// Send a verification code to the email address / phone number stored in the attribute
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
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    let (id, name) = path.into_inner();
    principal.validate_user_or_admin(&id)?;

    let (kind, value) = attr_contact(&id, &name).await?;
    let value = value.ok_or_else(|| {
        ErrorResponse::new(ErrorResponseType::BadRequest, "The attribute has no value")
    })?;
    if kind == ContactKind::Phone && !sms_available() {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "No SMS channel is configured",
        ));
    }

    let user = User::find(id).await?;
    let code = ContactVerification::start(&user.id, &attr_key(&name), &value).await?;
    let prefix = &RauthyConfig::get().vars.email.sub_prefix;
    let text = format!("Your {prefix} verification code is {code}. It is valid for 10 minutes.");

    match kind {
        ContactKind::Email => {
            channels::send_email_to(
                &value,
                &user.email_recipient_name(),
                "Verification code",
                text,
            )
            .await?;
        }
        ContactKind::Phone => {
            let msg = ChannelMessage::new(
                MessageChannel::Sms,
                MessagePurpose::Verification,
                ChannelRecipient {
                    user_id: user.id.clone(),
                    name: user.email_recipient_name(),
                    to: value,
                },
                None,
                text,
                None,
                None,
            );
            channels::send(MessageChannel::Sms, &msg).await?;
        }
    }

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

    let (_, value) = attr_contact(&id, &name).await?;
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

    let (_, value) = attr_contact(&id, &name).await?;
    let value = value.ok_or_else(|| {
        ErrorResponse::new(ErrorResponseType::BadRequest, "The attribute has no value")
    })?;

    ContactVerification::set_verified(&id, &attr_key(&name), &value, payload.verified).await?;
    Ok(HttpResponse::Ok().finish())
}
