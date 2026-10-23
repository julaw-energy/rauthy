use crate::ReqPrincipal;
use actix_web::web::{Json, Path};
use actix_web::{HttpResponse, get, post};
use chrono::Utc;
use rauthy_api_types::email_jobs::{
    ChannelInfo, EmailContentType, EmailJobFilterType, EmailJobRequest, EmailJobResponse,
    EmailTargets, MessageChannel,
};
use rauthy_common::sanitize_html::sanitize_html;
use rauthy_data::channels::ChannelDriver;
use rauthy_data::entity::email_jobs::{EmailJob, EmailJobFilter, EmailJobStatus};
use rauthy_data::settings;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use validator::Validate;

/// A delegated group admin may only see / cancel / send email jobs that target a
/// single group it manages. Any other filter (no filter, `not_in_group`, role filters, or a
/// group it does not manage) stays full-admin only.
fn group_admin_email_scope(principal: &ReqPrincipal, filter: &EmailJobFilter) -> bool {
    match filter {
        EmailJobFilter::InGroup(group) => principal.group_admin_manages_any([group.as_str()]),
        EmailJobFilter::Targets(t) => targets_group_admin_scope(principal, t),
        _ => false,
    }
}

/// Multi-select targets are only allowed for group admins if they consist of managed groups
/// exclusively, without any role / user entries or excludes.
fn targets_group_admin_scope(principal: &ReqPrincipal, t: &EmailTargets) -> bool {
    !t.groups.is_empty()
        && t.roles.is_empty()
        && t.users.is_empty()
        && t.not_roles.is_empty()
        && t.not_groups.is_empty()
        && t.groups
            .iter()
            .all(|g| principal.group_admin_manages_any([g.as_str()]))
}

/// Get all EmailJobs
///
/// **Permissions**
/// - rauthy_admin
/// - group admin (only jobs scoped to a group it manages)
#[utoipa::path(
    get,
    path = "/email",
    tag = "email",
    responses(
        (status = 200, description = "Ok", body = [EmailJobResponse]),
        (status = 400, description = "BadRequest"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
)]
#[get("/email")]
pub async fn get_email_jobs(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    // a full admin sees all jobs; a delegated group admin only sees jobs scoped to a group
    // it manages
    let full_admin = principal.is_admin();
    if full_admin {
        principal.validate_admin_session()?;
    } else {
        principal.validate_group_admin_session()?;
    }

    // TODO maybe accept a limit here?

    let resp = EmailJob::find_all()
        .await?
        .into_iter()
        .filter(|job| full_admin || group_admin_email_scope(&principal, &job.filter))
        .map(EmailJobResponse::from)
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(resp))
}

/// Get the message channels that are available for sending
///
/// **Permissions**
/// - rauthy_admin
/// - group admin
#[utoipa::path(
    get,
    path = "/email/channels",
    tag = "email",
    responses(
        (status = 200, description = "Ok", body = [ChannelInfo]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
)]
#[get("/email/channels")]
pub async fn get_channels(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    if principal.is_admin() {
        principal.validate_admin_session()?;
    } else {
        principal.validate_group_admin_session()?;
    }

    let resp = settings::available_channels()
        .into_iter()
        .map(|(channel, driver)| ChannelInfo {
            channel,
            driver: driver.as_str().to_string(),
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(resp))
}

/// Send out a custom email, SMS or WhatsApp message to users
///
/// The `channel` defaults to `email`. SMS and WhatsApp need a configured driver, only support
/// plain text, and skip users without a phone number.
///
/// **Permissions**
/// - rauthy_admin
/// - group admin (only an `in_group` filter for a group it manages)
#[utoipa::path(
    post,
    path = "/email",
    tag = "email",
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
)]
#[post("/email")]
pub async fn post_send_email(
    principal: ReqPrincipal,
    Json(mut payload): Json<EmailJobRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    // a delegated group admin may only send to a single group it manages
    if principal.is_admin() {
        principal.validate_admin_session()?;
    } else {
        principal.validate_group_admin_session()?;
        let allowed = match payload.targets.as_ref().filter(|t| !t.is_empty()) {
            Some(t) => targets_group_admin_scope(&principal, t),
            None => {
                matches!(payload.filter_type, EmailJobFilterType::InGroup)
                    && payload
                        .filter_value
                        .as_deref()
                        .map(|g| principal.group_admin_manages_any([g]))
                        .unwrap_or(false)
            }
        };
        if !allowed {
            return Err(ErrorResponse::new(
                ErrorResponseType::Forbidden,
                "Group admins may only send emails to groups they manage",
            ));
        }
    }
    payload.validate()?;

    if settings::channel(payload.channel).driver == ChannelDriver::Disabled {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            format!("Channel '{}' is not configured", payload.channel.as_str()),
        ));
    }
    if payload.channel != MessageChannel::Email && payload.content_type != EmailContentType::Text {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "SMS and WhatsApp only support plain text",
        ));
    }

    if payload.content_type == EmailContentType::HTML {
        payload.body = sanitize_html(&payload.body);
    }

    let job = EmailJob::insert(payload).await?;
    if job.scheduled.is_none() || job.scheduled.unwrap_or_default() <= Utc::now().timestamp() + 5 {
        job.spawn_task();
    }

    Ok(HttpResponse::Ok().finish())
}

/// Cancel a long-running E-Mail job early
///
/// **Permissions**
/// - rauthy_admin
/// - group admin (only jobs scoped to a group it manages)
#[utoipa::path(
    post,
    path = "/email/cancel/{id}",
    tag = "email",
    responses(
        (status = 200, description = "Ok"),
        (status = 400, description = "BadRequest"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
)]
#[post("/email/cancel/{id}")]
pub async fn post_email_job_cancel(
    principal: ReqPrincipal,
    id: Path<i64>,
) -> Result<HttpResponse, ErrorResponse> {
    let full_admin = principal.is_admin();
    if full_admin {
        principal.validate_admin_session()?;
    } else {
        principal.validate_group_admin_session()?;
    }

    let mut job = EmailJob::find(id.into_inner()).await?;
    // a group admin may only cancel a job scoped to a group it manages
    if !full_admin && !group_admin_email_scope(&principal, &job.filter) {
        return Err(ErrorResponse::new(
            ErrorResponseType::Forbidden,
            "Group admins may only cancel jobs scoped to a group they manage",
        ));
    }
    if job.status != EmailJobStatus::Open {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            "Cannot cancel a non-open job",
        ));
    }
    job.cancel().await?;

    Ok(HttpResponse::Ok().finish())
}
