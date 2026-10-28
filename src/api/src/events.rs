use crate::ReqPrincipal;
use actix_web::web::Path;
use actix_web::web::{Json, Query};
use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, put};
use actix_web_lab::sse;
use chrono::Utc;
use rauthy_api_types::events::{
    AuditEntryResponse, AuditListParams, AuditSummaryParams, AuditSummaryResponse,
    AuditUserResponse, EventResponse, EventsListenParams, EventsReadState, EventsRequest,
};
use rauthy_common::utils::{get_rand, real_ip_from_req};
use rauthy_data::entity::api_keys::{AccessGroup, AccessRights};
use rauthy_data::entity::audit_log::AuditLog;
use rauthy_data::entity::audit_users::AuditUser;
use rauthy_data::entity::event_read_state::EventReadState;
use rauthy_data::events::event::Event;
use rauthy_data::events::listener::EventRouterMsg;
use rauthy_data::rauthy_config::RauthyConfig;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use std::time::Duration;
use tokio::sync::mpsc;
use validator::Validate;

/// Get events
#[utoipa::path(
    post,
    path = "/events",
    tag = "events",
    request_body = EventsRequest,
    responses(
        (status = 200, description = "Ok", body = [EventResponse]),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/events")]
pub async fn post_events(
    principal: ReqPrincipal,
    Json(payload): Json<EventsRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    // group admins get a read-only view of events for user debugging
    principal.validate_api_key_or_group_admin(AccessGroup::Events, AccessRights::Read)?;
    payload.validate()?;

    let events = Event::find_all(
        payload.from,
        payload.until.unwrap_or_else(|| Utc::now().timestamp()),
        payload.level.into(),
        payload.typ.map(|t| t.into()),
    )
    .await?
    .into_iter()
    .map(EventResponse::from)
    .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(events))
}

/// Listen to the Events SSE stream
#[utoipa::path(
    get,
    path = "/events/stream",
    tag = "events",
    params(EventsListenParams),
    responses(
        (status = 200, description = "Ok", body = [EventResponse]),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[get("/events/stream")]
pub async fn sse_events(
    principal: ReqPrincipal,
    params: Query<EventsListenParams>,
    req: HttpRequest,
) -> Result<impl Responder, ErrorResponse> {
    // group admins get a read-only event stream for user debugging
    principal.validate_api_key_or_group_admin(AccessGroup::Events, AccessRights::Read)?;
    params.validate()?;

    let ip = real_ip_from_req(&req)?.to_string();
    // each SSE connection gets a random id as its routing key, since multiple connections can
    // share the same remote IP (e.g. multiple browser tabs) and must not replace each other
    let client_id = get_rand(16);
    // who is listening: the session user or the API key name, for router logging
    let user_id = principal.session.as_ref().and_then(|s| s.user_id.clone());
    let api_key_name = principal.api_key.as_ref().map(|k| k.name.clone());
    let params = params.into_inner();
    let (tx, rx) = mpsc::channel(10);

    let level = params.level.map(|l| l.into()).unwrap_or_default();

    // the read marker of this user goes first, so the client can count right from the start
    let read_updates = params.read.unwrap_or(false) && user_id.is_some();
    if read_updates && let Some(uid) = &user_id {
        let read_until = EventReadState::find(uid).await?;
        let first = sse::Event::Data(
            sse::Data::new(format!(r#"{{"read_until":{read_until}}}"#)).event("read"),
        );
        // the channel has room for it, nothing is registered yet
        let _ = tx.try_send(first);
    }

    if let Err(err) = RauthyConfig::get()
        .tx_events_router
        .send_async(EventRouterMsg::ClientReg {
            client_id,
            ip,
            user_id,
            api_key_name,
            tx,
            latest: params.latest,
            level,
            read_updates,
        })
        .await
    {
        Err(ErrorResponse::new(
            ErrorResponseType::Internal,
            format!("Cannot register SSE client: {err:?}"),
        ))
    } else {
        Ok(sse::Sse::from_infallible_receiver(rx)
            .with_keep_alive(RauthyConfig::get().vars.server.see_keep_alive)
            .with_retry_duration(Duration::from_secs(10)))
    }
}

/// Who did what, newest first
///
/// The actor is the id of a user, or `system` for what the background tasks do on their own.
/// Page back with `before`, the `at` (milliseconds) of the last entry you got.
///
/// **Permissions**
/// - an admin session or an API key with `Events` read
#[utoipa::path(
    get,
    path = "/events/audit",
    tag = "events",
    params(AuditListParams),
    responses(
        (status = 200, description = "Ok", body = [AuditEntryResponse]),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/events/audit")]
pub async fn get_audit_log(
    principal: ReqPrincipal,
    params: Query<AuditListParams>,
) -> Result<HttpResponse, ErrorResponse> {
    // the actors are named by id, so this is for admins and not for group admins
    principal.validate_api_key_or_admin_session(AccessGroup::Events, AccessRights::Read)?;
    params.validate()?;

    let before = params.before.unwrap_or(i64::MAX);
    let limit = params.limit.unwrap_or(50) as i64;
    let entries = AuditLog::find(params.target_id.as_deref(), before, limit).await?;
    let labels = AuditLog::labels(
        entries.iter().map(|e| e.actor.as_str()).chain(
            entries
                .iter()
                .filter(|e| e.target_type == "users")
                .filter_map(|e| e.target_id.as_deref()),
        ),
    )
    .await;
    let entries: Vec<AuditEntryResponse> = entries
        .into_iter()
        .map(|e| AuditEntryResponse {
            actor_label: labels.get(&e.actor).cloned(),
            target_label: (e.target_type == "users")
                .then(|| e.target_id.as_ref().and_then(|t| labels.get(t)).cloned())
                .flatten(),
            id: e.id,
            at: e.at,
            actor: e.actor,
            action: e.action,
            target_type: e.target_type,
            target_id: e.target_id,
            details: e.details,
        })
        .collect();
    Ok(HttpResponse::Ok().json(entries))
}

/// Who created something, and who changed it last
///
/// Read from the audit log: what was created before the audit log existed has no creator.
///
/// **Permissions**
/// - an admin session or an API key with `Events` read
#[utoipa::path(
    get,
    path = "/events/audit/summary",
    tag = "events",
    params(AuditSummaryParams),
    responses(
        (status = 200, description = "Ok", body = AuditSummaryResponse),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/events/audit/summary")]
pub async fn get_audit_summary(
    principal: ReqPrincipal,
    params: Query<AuditSummaryParams>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_api_key_or_admin_session(AccessGroup::Events, AccessRights::Read)?;
    params.validate()?;

    let s = AuditLog::summary(&params.target_id).await?;
    let labels = AuditLog::labels(
        s.created
            .iter()
            .chain(s.modified.iter())
            .map(|e| e.actor.as_str()),
    )
    .await;
    let label = |e: &Option<rauthy_data::entity::audit_log::AuditEntry>| {
        e.as_ref().and_then(|e| labels.get(&e.actor)).cloned()
    };
    Ok(HttpResponse::Ok().json(AuditSummaryResponse {
        created_by_label: label(&s.created),
        modified_by_label: label(&s.modified),
        created_by: s.created.as_ref().map(|e| e.actor.clone()),
        created_at: s.created.as_ref().map(|e| e.at),
        modified_by: s.modified.as_ref().map(|e| e.actor.clone()),
        modified_at: s.modified.as_ref().map(|e| e.at),
    }))
}

/// The users who were deleted, newest first, with what is kept of them
///
/// **Permissions**
/// - rauthy_admin session
#[utoipa::path(
    get,
    path = "/events/audit/users",
    tag = "events",
    responses(
        (status = 200, description = "Ok", body = [AuditUserResponse]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/events/audit/users")]
pub async fn get_audit_users(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;

    let users: Vec<AuditUserResponse> = AuditUser::find_recent(200)
        .await?
        .into_iter()
        .map(|u| AuditUserResponse {
            id: u.id,
            email: u.email,
            given_name: u.given_name,
            family_name: u.family_name,
            preferred_username: u.preferred_username,
            phone: u.phone,
            deleted_at: u.deleted_at,
            anonymized: u.anonymized,
        })
        .collect();
    Ok(HttpResponse::Ok().json(users))
}

/// Wipes the personal data kept for a deleted user
///
/// When a user is deleted, their email, names, username and phone are kept so that the audit log
/// can still name them. This removes them, only the id stays. It does nothing to users who exist.
///
/// **Permissions**
/// - rauthy_admin session
#[utoipa::path(
    delete,
    path = "/events/audit/users/{id}",
    tag = "events",
    responses(
        (status = 204, description = "Ok"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "NotFound", body = ErrorResponse),
    ),
)]
#[delete("/events/audit/users/{id}")]
pub async fn delete_audit_user(
    principal: ReqPrincipal,
    id: Path<String>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;
    if AuditUser::anonymize(&id).await? {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(ErrorResponse::new(
            ErrorResponseType::NotFound,
            "No deleted user with this id is kept",
        ))
    }
}

/// How far the logged-in user has read the events
///
/// **Permissions**
/// - an admin or group admin session
#[utoipa::path(
    get,
    path = "/events/read",
    tag = "events",
    responses(
        (status = 200, description = "Ok", body = EventsReadState),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/events/read")]
pub async fn get_events_read(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_api_key_or_group_admin(AccessGroup::Events, AccessRights::Read)?;
    let user_id = session_user_id(&principal)?;

    Ok(HttpResponse::Ok().json(EventsReadState {
        read_until: EventReadState::find(&user_id).await?,
    }))
}

/// Mark the events as read up to a point
///
/// The marker only moves forward. Every open event stream of the user that asked for it gets the
/// new value as the SSE event `read`, so the unread count is the same on all their devices.
///
/// **Permissions**
/// - an admin or group admin session
#[utoipa::path(
    put,
    path = "/events/read",
    tag = "events",
    request_body = EventsReadState,
    responses(
        (status = 200, description = "Ok, the marker after the update", body = EventsReadState),
        (status = 400, description = "BadRequest", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[put("/events/read")]
pub async fn put_events_read(
    principal: ReqPrincipal,
    Json(payload): Json<EventsReadState>,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_api_key_or_group_admin(AccessGroup::Events, AccessRights::Read)?;
    payload.validate()?;
    let user_id = session_user_id(&principal)?;

    let read_until = EventReadState::advance(&user_id, payload.read_until).await?;
    let _ = RauthyConfig::get()
        .tx_events_router
        .send_async(EventRouterMsg::ReadMarker {
            user_id,
            read_until,
        })
        .await;

    Ok(HttpResponse::Ok().json(EventsReadState { read_until }))
}

/// The read marker belongs to a person, so an API key has none
fn session_user_id(principal: &ReqPrincipal) -> Result<String, ErrorResponse> {
    principal
        .session
        .as_ref()
        .and_then(|s| s.user_id.clone())
        .ok_or_else(|| {
            ErrorResponse::new(
                ErrorResponseType::Forbidden,
                "The read state needs a user session",
            )
        })
}

/// Create a TEST Event
#[utoipa::path(
    post,
    path = "/events/test",
    tag = "events",
    responses(
        (status = 200, description = "Ok"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[post("/events/test")]
pub async fn post_event_test(
    principal: ReqPrincipal,
    req: HttpRequest,
) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_api_key_or_admin_session(AccessGroup::Events, AccessRights::Create)?;

    Event::test(real_ip_from_req(&req)?).send().await?;

    #[cfg(debug_assertions)]
    if RauthyConfig::get().vars.dev.dev_mode {
        use rauthy_data::entity::failed_scim_tasks::ScimAction;
        use std::net::IpAddr;

        // in debug + dev mode, we want to generate one event of each type
        // as a helper for UI development
        let ip = "0.0.0.0".parse::<IpAddr>().unwrap();

        Event::backchannel_logout_failed("dummy_client", "dummy_user", 3)
            .send()
            .await?;
        Event::invalid_login(1, ip.to_string()).send().await?;
        Event::brute_force(ip.to_string()).send().await?;
        Event::force_logout("dummy@example.com".to_string())
            .send()
            .await?;
        Event::ip_blacklisted(Utc::now(), ip.to_string())
            .send()
            .await?;
        Event::ip_blacklist_removed(ip.to_string()).send().await?;
        Event::cred_stuff(ip.to_string()).send().await?;
        Event::new_user("test@dummy".to_string(), ip.to_string())
            .send()
            .await?;
        Event::new_rauthy_admin("test-admin@dummy".to_string(), ip.to_string())
            .send()
            .await?;
        Event::new_rauthy_version("https://github.com/sebadob/rauthy/releases".to_string())
            .send()
            .await?;
        Event::jwks_rotated().send().await?;
        Event::user_login_revoke(
            "alfred@batcave.io",
            "123.123.123.123".parse().unwrap(),
            Some("Gotham City".to_string()),
        )
        .send()
        .await?;
        Event::rauthy_started().send().await?;
        Event::rauthy_healthy().send().await?;
        Event::rauthy_unhealthy_cache().send().await?;
        Event::rauthy_unhealthy_db().send().await?;
        Event::scim_task_failed("dummy_client", &ScimAction::GroupsSync, 3)
            .send()
            .await?;
        Event::secrets_migrated(ip).send().await?;
        Event::suspicious_request(
            "/.git/config",
            "123.123.123.123".parse().unwrap(),
            Some("Gotham City".to_string()),
        )
        .send()
        .await?;
        Event::test(ip).send().await?;

        let old_email = "old@mail";
        let new_mail = "new@mail";
        let text = format!("{old_email} -> {new_mail}");
        let text_admin = format!("Change by admin: {old_email} -> {new_mail}");
        Event::user_email_change(text, Some(ip)).send().await?;
        Event::user_email_change(text_admin, Some(ip))
            .send()
            .await?;

        let text = format!("Reset via Password Reset Form: {}", "dummy@mail");
        let text_admin = format!("Reset done by admin for user {}", "dummy@mail");
        Event::user_password_reset(text, Some(ip.to_string()))
            .send()
            .await?;
        Event::user_password_reset(text_admin, Some(ip.to_string()))
            .send()
            .await?;
    }

    Ok(HttpResponse::Ok().finish())
}
