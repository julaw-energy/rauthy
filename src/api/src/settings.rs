use crate::ReqPrincipal;
use actix_web::web::{Json, Path};
use actix_web::{HttpResponse, get, post, put};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::settings::{
    AppNameResponse, ChannelSettingsRequest, GeneralSettingsRequest, GeneralSettingsResponse,
    SettingsResponse, SettingsTestResponse, SmtpSettingsRequest,
};
use rauthy_data::channels;
use rauthy_data::email::mailer;
use rauthy_data::entity::users::User;
use rauthy_data::settings::{self, KEY_GENERAL, KEY_SMTP};
use rauthy_error::{ErrorResponse, ErrorResponseType};

fn channel_from_path(name: &str) -> Result<MessageChannel, ErrorResponse> {
    name.parse::<MessageChannel>()
        .map_err(|_| ErrorResponse::new(ErrorResponseType::BadRequest, "Unknown channel"))
}

/// GET the settings that are managed in the database: the SMTP server and the message channels
///
/// Secrets (the SMTP password, API keys) are never returned, only whether they are set.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/settings",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = SettingsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/settings")]
pub async fn get_settings(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;

    Ok(HttpResponse::Ok().json(SettingsResponse {
        general: GeneralSettingsResponse {
            app_name: settings::app_name(),
            from_db: settings::general_from_db(),
        },
        smtp: settings::smtp_view(),
        smtp_configured: settings::smtp().is_configured(),
        channels: [
            MessageChannel::Email,
            MessageChannel::Sms,
            MessageChannel::Whatsapp,
        ]
        .into_iter()
        .map(settings::channel_view)
        .collect(),
    }))
}

/// The name of this service, as users should see it. Public.
#[utoipa::path(
    get,
    path = "/app_name",
    tag = "settings",
    responses((status = 200, description = "Ok", body = AppNameResponse)),
)]
#[get("/app_name")]
pub async fn get_app_name() -> HttpResponse {
    HttpResponse::Ok()
        // the default would be `no-store`, but this is asked on every page load
        .insert_header(("cache-control", "public, max-age=30"))
        .json(AppNameResponse {
            name: settings::app_name(),
        })
}

/// Change the name of this service. Applies immediately, no restart needed.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/general",
    tag = "settings",
    request_body = GeneralSettingsRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/general")]
pub async fn put_settings_general(
    principal: ReqPrincipal,
    Json(payload): Json<GeneralSettingsRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_general(&payload.app_name).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Change the SMTP settings. They apply immediately, no restart needed.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/smtp",
    tag = "settings",
    request_body = SmtpSettingsRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/smtp")]
pub async fn put_settings_smtp(
    principal: ReqPrincipal,
    Json(payload): Json<SmtpSettingsRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_smtp(payload).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Send a test E-Mail to the admin who clicks the button, with the current SMTP settings
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/smtp/test",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = SettingsTestResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/settings/smtp/test")]
pub async fn post_settings_smtp_test(
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;

    let user = User::find(principal.user_id()?.to_string()).await?;
    mailer::send_test(&user.email, &user.email_recipient_name()).await?;

    Ok(HttpResponse::Ok().json(SettingsTestResponse {
        message: format!("Test E-Mail sent to {}", user.email),
    }))
}

/// Change the config of a message channel (`email`, `sms` or `whatsapp`). Applies immediately.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/channel/{channel}",
    tag = "settings",
    request_body = ChannelSettingsRequest,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/channel/{channel}")]
pub async fn put_settings_channel(
    path: Path<String>,
    principal: ReqPrincipal,
    Json(payload): Json<ChannelSettingsRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_channel(channel_from_path(&path)?, payload).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Check that the notification agent of a channel can be reached
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/channel/{channel}/test",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = SettingsTestResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/settings/channel/{channel}/test")]
pub async fn post_settings_channel_test(
    path: Path<String>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let message = channels::test(channel_from_path(&path)?).await?;
    Ok(HttpResponse::Ok().json(SettingsTestResponse { message }))
}

/// Replace a settings group with the values of the config file / env as they are right now.
///
/// `group` is `general`, `smtp`, `channel.email`, `channel.sms` or `channel.whatsapp`.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/import/{group}",
    tag = "settings",
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/settings/import/{group}")]
pub async fn post_settings_import(
    path: Path<String>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let group = path.into_inner();
    if group != KEY_SMTP && group != KEY_GENERAL && !group.starts_with("channel.") {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "Unknown settings group",
        ));
    }
    settings::import_from_file(&group).await?;
    Ok(HttpResponse::Ok().finish())
}
