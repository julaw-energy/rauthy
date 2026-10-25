use crate::ReqPrincipal;
use actix_web::web::{Json, Path, Query};
use actix_web::{HttpResponse, delete, get, post, put};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::message_templates::{
    TemplateDetail, TemplateInfo, TemplatePreviewResponse, TemplateSaveRequest,
    TemplateTestRequest, TemplateTestResponse,
};
use rauthy_data::channels::{self, MessagePurpose};
use rauthy_data::email::mailer::{EMail, EmailType};
use rauthy_data::email::mailer_callback::EMailCallback;
use rauthy_data::entity::users::User;
use rauthy_data::message_templates::{self as mt, Kind, TemplateDef};
use rauthy_data::rauthy_config::RauthyConfig;
use rauthy_data::settings;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct LangQuery {
    pub lang: String,
}

fn def_of(id: &str) -> Result<&'static TemplateDef, ErrorResponse> {
    mt::def(id).ok_or_else(|| ErrorResponse::new(ErrorResponseType::NotFound, "Unknown template"))
}

/// List the message templates that can be edited
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/settings/templates",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = [TemplateInfo]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/settings/templates")]
pub async fn get_templates(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let list: Vec<TemplateInfo> = mt::defs().iter().map(mt::info).collect();
    Ok(HttpResponse::Ok().json(list))
}

/// The texts of a template in one language, and the built-in ones
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/settings/templates/{id}",
    tag = "settings",
    params(("lang" = String, Query, description = "language code, or `default`")),
    responses(
        (status = 200, description = "Ok", body = TemplateDetail),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[get("/settings/templates/{id}")]
pub async fn get_template(
    path: Path<String>,
    query: Query<LangQuery>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let d = def_of(&path)?;
    if !mt::lang_is_valid(d, &query.lang) {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "Unknown language",
        ));
    }
    Ok(HttpResponse::Ok().json(mt::detail(d, &query.lang)))
}

/// Save the texts of a template
///
/// The texts are checked first, a template with an error is not saved and the one in use stays
/// as it is. On success it is used by the next message that is sent, without a restart.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    put,
    path = "/settings/templates/{id}",
    tag = "settings",
    request_body = TemplateSaveRequest,
    responses(
        (status = 200, description = "Ok", body = TemplateTestResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[put("/settings/templates/{id}")]
pub async fn put_template(
    path: Path<String>,
    principal: ReqPrincipal,
    Json(payload): Json<TemplateSaveRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let d = def_of(&path)?;
    let warnings = mt::save(d, &payload.lang, payload.values).await?;
    Ok(HttpResponse::Ok().json(TemplateTestResponse {
        message: "Saved".to_string(),
        warnings,
    }))
}

/// Go back to the built-in texts of a template
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    delete,
    path = "/settings/templates/{id}",
    tag = "settings",
    params(("lang" = String, Query, description = "language code, or `default`")),
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[delete("/settings/templates/{id}")]
pub async fn delete_template(
    path: Path<String>,
    query: Query<LangQuery>,
    principal: ReqPrincipal,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    mt::reset(def_of(&path)?, &query.lang).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Render unsaved texts with sample values, without sending or saving anything
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/templates/{id}/preview",
    tag = "settings",
    request_body = TemplateTestRequest,
    responses(
        (status = 200, description = "Ok", body = TemplatePreviewResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[post("/settings/templates/{id}/preview")]
pub async fn post_template_preview(
    path: Path<String>,
    principal: ReqPrincipal,
    Json(payload): Json<TemplateTestRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let d = def_of(&path)?;
    let user = User::find(principal.user_id()?.to_string()).await?;
    let res = mt::preview(d, &payload.lang, &payload.values, &user).await?;
    Ok(HttpResponse::Ok().json(res))
}

/// Send unsaved texts to the admin who is logged in, to see them as a user would
///
/// An `email` template goes to the email address of the admin. A `message` template goes through
/// the chosen channel to the value of the admin that the channel is tied to, for example the phone
/// number. If the admin has no such value, the test fails. Nothing is saved.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    post,
    path = "/settings/templates/{id}/test",
    tag = "settings",
    request_body = TemplateTestRequest,
    responses(
        (status = 200, description = "Ok", body = TemplateTestResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[post("/settings/templates/{id}/test")]
pub async fn post_template_test(
    path: Path<String>,
    principal: ReqPrincipal,
    Json(payload): Json<TemplateTestRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    let d = def_of(&path)?;
    let user = User::find(principal.user_id()?.to_string()).await?;
    let preview = mt::preview(d, &payload.lang, &payload.values, &user).await?;

    let sent_to = match d.kind {
        Kind::Email => {
            let req = EMail {
                typ: EmailType::Custom,
                recipient_name: user.email_recipient_name(),
                address: user.email.clone(),
                subject: preview.subject.clone(),
                text: Some(preview.text.clone()),
                html: preview.html.clone(),
            };
            RauthyConfig::get()
                .tx_email
                .send_timeout((req, EMailCallback::None), Duration::from_secs(10))
                .await
                .map_err(|_| {
                    ErrorResponse::new(ErrorResponseType::Timeout, "email send timeout")
                })?;
            user.email.clone()
        }
        Kind::Message => {
            let channel = match payload.channel.as_deref().filter(|c| !c.is_empty()) {
                Some(c) => c.parse::<MessageChannel>().map_err(|_| {
                    ErrorResponse::new(ErrorResponseType::BadRequest, "Unknown channel")
                })?,
                None => MessageChannel::email(),
            };
            let cfg = settings::channel(&channel);
            // fails when the admin has no value of the kind this channel delivers to
            let to = channels::resolve_recipient(&user, &cfg.recipient).await?;
            channels::deliver(
                &channel,
                &user,
                to.clone(),
                MessagePurpose::Verification,
                &preview.subject,
                preview.text.clone(),
            )
            .await?;
            to
        }
    };

    Ok(HttpResponse::Ok().json(TemplateTestResponse {
        message: format!("Test sent to {sent_to}"),
        warnings: preview.warnings,
    }))
}
