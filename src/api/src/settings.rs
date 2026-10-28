use crate::ReqPrincipal;
use actix_web::web::{Json, Path};
use actix_web::{HttpResponse, delete, get, post, put};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::settings::{
    AppNameResponse, ChannelSettingsRequest, GeneralSettingsRequest, GeneralSettingsResponse,
    LifetimesSettings, NewUsersSettings, NewUsersSettingsResponse, ReminderSettings,
    ReminderSettingsResponse, SettingsResponse, SettingsTestResponse, SmtpSettingsRequest,
    UserValuesSettings,
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
        user_values: settings::user_values_view(),
        lifetimes: settings::lifetimes_view(),
        general: GeneralSettingsResponse {
            app_name: settings::app_name(),
            from_db: settings::general_from_db(),
        },
        smtp: settings::smtp_view(),
        smtp_configured: settings::smtp().is_configured(),
        channels: settings::all_channels()
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

/// Change how long sessions, refresh tokens and email links stay valid. Applies immediately.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/lifetimes",
    tag = "settings",
    request_body = LifetimesSettings,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/lifetimes")]
pub async fn put_settings_lifetimes(
    principal: ReqPrincipal,
    Json(payload): Json<LifetimesSettings>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_lifetimes(payload).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Change which profile fields are shown or required. Applies immediately, no restart needed.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/user_values",
    tag = "settings",
    request_body = UserValuesSettings,
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/user_values")]
pub async fn put_settings_user_values(
    principal: ReqPrincipal,
    Json(payload): Json<UserValuesSettings>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_user_values_request(payload).await?;
    Ok(HttpResponse::Ok().finish())
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

/// Create or change a message channel. A new name creates a custom channel. Applies immediately.
///
/// `recipient` ties the channel to the user value it delivers to: `email`, `phone` or
/// `attr:<name>` for a custom attribute. A user without that value gets no message through the
/// channel.
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
    let message = channels::test(&channel_from_path(&path)?).await?;
    Ok(HttpResponse::Ok().json(SettingsTestResponse { message }))
}

/// Delete a custom message channel. The built-in channels can only be disabled.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    delete,
    path = "/settings/channel/{channel}",
    tag = "settings",
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[delete("/settings/channel/{channel}")]
pub async fn delete_settings_channel(
    path: Path<String>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::delete_channel(&channel_from_path(&path)?).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Send a test message through a channel to the admin who is logged in
///
/// The message goes to the value of the admin the channel is tied to (email address, phone
/// number or attribute). An admin who has none fails the test.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/channel/{channel}/send_test",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = SettingsTestResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/settings/channel/{channel}/send_test")]
pub async fn post_settings_channel_send_test(
    path: Path<String>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let channel = channel_from_path(&path)?;

    let user = User::find(principal.user_id()?.to_string()).await?;
    let cfg = settings::channel(&channel);
    let to = channels::resolve_recipient(&user, &cfg.recipient).await?;
    let text = format!("Test message from {}.", settings::app_name());
    let delivery = channels::deliver(
        &channel,
        &user,
        to.clone(),
        channels::MessagePurpose::Security,
        "Test message",
        text,
    )
    .await?;

    // what the other side answered, so an admin sees that it really arrived there
    let mut message = format!("Test message sent to {to}");
    if let Some(status) = delivery.status {
        message.push_str(&format!(". The endpoint answered {status}"));
        let answer = delivery.answer.trim();
        if !answer.is_empty() {
            message.push_str(&format!(": {answer}"));
        }
    }
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
    if group != KEY_SMTP
        && group != KEY_GENERAL
        && group != settings::KEY_USER_VALUES
        && group != settings::KEY_LIFETIMES
        && !group.starts_with("channel.")
    {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "Unknown settings group",
        ));
    }
    settings::import_from_file(&group).await?;
    Ok(HttpResponse::Ok().finish())
}

/// The roles and groups every new user gets
///
/// New users are the ones who register themselves and the ones created by their first login
/// through an auth provider. The response also lists the roles and groups to choose from.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/settings/new_users",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = NewUsersSettingsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/settings/new_users")]
pub async fn get_settings_new_users(
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    Ok(HttpResponse::Ok().json(settings::new_users_view().await?))
}

/// Sets the roles and groups every new user gets
///
/// `rauthy_admin` and the delegated admin roles cannot be set, they would make every new user an
/// admin. A role or group that is deleted later is skipped when a user is created.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/new_users",
    tag = "settings",
    request_body = NewUsersSettings,
    responses(
        (status = 200, description = "Ok", body = NewUsersSettingsResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/new_users")]
pub async fn put_settings_new_users(
    principal: ReqPrincipal,
    Json(payload): Json<NewUsersSettings>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_new_users(payload).await?;
    Ok(HttpResponse::Ok().json(settings::new_users_view().await?))
}

/// The reminder for users who have not logged in for a long time
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/settings/reminders",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = ReminderSettingsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/settings/reminders")]
pub async fn get_settings_reminders(
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    Ok(HttpResponse::Ok().json(settings::reminders_view().await?))
}

/// Sets the reminder for users who have not logged in for a long time
///
/// While `enabled` is off, nothing is sent. Once it is on, users who have been away for
/// `after_days` get a reminder, then another one every `after_days`, at most `max_reminders` in a
/// row, until they log in again. The response says how many users are due right now.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/reminders",
    tag = "settings",
    request_body = ReminderSettings,
    responses(
        (status = 200, description = "Ok", body = ReminderSettingsResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/settings/reminders")]
pub async fn put_settings_reminders(
    principal: ReqPrincipal,
    Json(payload): Json<ReminderSettings>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    settings::save_reminders(payload).await?;
    Ok(HttpResponse::Ok().json(settings::reminders_view().await?))
}

/// Sends the reminder, with the wording that is set now, to the logged-in admin
///
/// Nothing is sent to anyone else and nothing is recorded.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/reminders/test",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = SettingsTestResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/settings/reminders/test")]
pub async fn post_settings_reminders_test(
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;

    let user = User::find(principal.user_id()?.to_string()).await?;
    let s = settings::reminders();
    rauthy_data::email::inactivity::send_inactivity_reminder(&user, s.after_days as i64, &s)
        .await?;
    Ok(HttpResponse::Ok().json(SettingsTestResponse {
        message: format!("The reminder was sent to {}", user.email),
    }))
}
