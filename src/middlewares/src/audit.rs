//! Writes an audit entry for every successful change made through the API: who did it (a session
//! user or an API key), when, and to what. It never looks at what was sent, only at the route.
//!
//! The entry says `create`, `update`, `delete` or `action` (a change that is none of these, like
//! a password reset), the type of the target (`users`, `clients`, ...) and its id. For a create,
//! the id is read from the answer, where the new thing is named by its `id` or `name`.
//!
//! Only what changes something is listed, see [`audited`]. The writing happens in the background,
//! it does not add to the time a request takes.

use actix_web::body::{BodySize, BoxBody, MessageBody, to_bytes};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::http::Method;
use actix_web::{Error, HttpMessage};
use futures::future::LocalBoxFuture;
use rauthy_data::entity::audit_log::AuditLog;
use rauthy_data::entity::principal::Principal;
use std::future::{Ready, ready};
use std::rc::Rc;

/// A create answer is small. Bigger or streamed answers are left alone.
const MAX_CREATE_BODY: u64 = 64 * 1024;

pub struct RauthyAuditMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RauthyAuditMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuditMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuditMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuditMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuditMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        // decided before the request is handled, a request that is not audited costs two checks
        let who = if is_change(req.method()) {
            req.path()
                .strip_prefix("/auth/v1")
                .filter(|p| audited(p))
                .and_then(|_| actor_of(&req))
        } else {
            None
        };

        Box::pin(async move {
            let res = service.call(req).await?;
            let Some(actor) = who else {
                return Ok(res.map_into_boxed_body());
            };
            if !res.status().is_success() {
                return Ok(res.map_into_boxed_body());
            }

            let http_req = res.request().clone();
            let method = http_req.method().clone();
            let rest = http_req
                .path()
                .strip_prefix("/auth/v1")
                .unwrap_or_default()
                .to_string();
            let pattern = http_req
                .match_pattern()
                .map(|p| p.strip_prefix("/auth/v1").unwrap_or(&p).to_string())
                .unwrap_or_else(|| rest.clone());
            let param = http_req
                .match_info()
                .iter()
                .next()
                .map(|(_, v)| v.to_string());
            let (target_type, mut target_id) = target_of(&rest, param);
            let action = action_of(&method, &pattern);

            let res = if action == "create" && target_id.is_none() {
                // the new thing is named in the answer
                let (req, res) = res.into_parts();
                let (head, body) = res.into_parts();
                match body.size() {
                    BodySize::Sized(n) if n <= MAX_CREATE_BODY => match to_bytes(body).await {
                        Ok(bytes) => {
                            target_id = id_from_body(&bytes);
                            ServiceResponse::new(req, head.set_body(bytes).map_into_boxed_body())
                        }
                        Err(_) => {
                            return Err(actix_web::error::ErrorInternalServerError(
                                "The answer could not be read",
                            ));
                        }
                    },
                    _ => ServiceResponse::new(req, head.set_body(body).map_into_boxed_body()),
                }
            } else {
                res.map_into_boxed_body()
            };

            let details = format!("{method} {pattern}");
            actix_web::rt::spawn(async move {
                AuditLog::record_or_log(
                    &actor,
                    action,
                    &target_type,
                    target_id.as_deref(),
                    Some(details),
                )
                .await;
            });
            Ok(res)
        })
    }
}

fn is_change(m: &Method) -> bool {
    !matches!(*m, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn actor_of(req: &ServiceRequest) -> Option<String> {
    let ext = req.extensions();
    let p = ext.get::<Principal>()?;
    if let Some(id) = p.session.as_ref().and_then(|s| s.user_id.as_ref()) {
        return Some(id.clone());
    }
    p.api_key.as_ref().map(|k| format!("api_key:{}", k.name))
}

/// Routes (below `/auth/v1`) that change something an admin or a user would want a record of.
/// Logins, token requests, the steps of a passkey login, mails that only send a code and test
/// buttons are not.
pub fn audited(path: &str) -> bool {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let Some(first) = segs.first() else {
        return false;
    };
    let listed = matches!(
        *first,
        "api_keys"
            | "backup"
            | "blacklist"
            | "clients"
            | "clients_dyn"
            | "email"
            | "encryption"
            | "groups"
            | "kv"
            | "pam"
            | "password_policy"
            | "providers"
            | "roles"
            | "scopes"
            | "sessions"
            | "settings"
            | "theme"
            | "tos"
            | "users"
    );
    if !listed {
        return false;
    }
    let last = segs.last().copied().unwrap_or_default();
    // tests, previews and the steps that only send or check a code change nothing
    if matches!(
        last,
        "test" | "send_test" | "preview" | "start" | "validate_uri"
    ) {
        return false;
    }
    match *first {
        // the login steps of the host agents and of passkeys, registration and password reset
        "pam" => {
            !matches!(
                segs.get(1).copied(),
                Some("login" | "preflight" | "getent" | "mfa" | "password")
            ) && last != "whoami"
        }
        "providers" => !matches!(segs.get(1).copied(), Some("callback" | "login" | "lookup")),
        "users" => {
            !matches!(
                segs.get(1).copied(),
                Some(
                    "register"
                        | "request_reset"
                        | "otp_start"
                        | "otp_finish"
                        | "otp_resend"
                        | "webauthn_start"
                        | "webauthn_finish"
                )
            ) && !(segs.get(2) == Some(&"webauthn") && segs.get(3) == Some(&"auth"))
                && segs.get(2) != Some(&"mfa_token")
        }
        "tos" => !matches!(last, "accept" | "deny"),
        _ => true,
    }
}

/// `(type, id)` of what a change is about. The id is the first value of the route, or the second
/// part of the path (`/settings/reminders`), and none for the creation of something.
fn target_of(path: &str, param: Option<String>) -> (String, Option<String>) {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let typ = segs.first().copied().unwrap_or_default().to_string();
    if param.is_some() {
        return (typ, param);
    }
    // `/users/attr`, `/kv/ns` and the like create something, the rest names its target
    let creates = segs.len() == 1 || (segs.len() == 2 && matches!(segs[1], "attr" | "ns"));
    if creates && !is_named_settings(&typ) {
        return (typ, None);
    }
    (typ, segs.get(1).map(|s| s.to_string()))
}

fn is_named_settings(typ: &str) -> bool {
    typ == "settings"
}

fn action_of(method: &Method, pattern: &str) -> &'static str {
    let collection = pattern.split('/').filter(|s| !s.is_empty()).count() == 1
        || matches!(pattern, "/users/attr" | "/kv/ns" | "/clients_dyn");
    match *method {
        Method::POST if collection => "create",
        Method::PUT | Method::PATCH if !pattern.ends_with("/secret") => "update",
        Method::DELETE => "delete",
        _ => "action",
    }
}

fn id_from_body(bytes: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    ["id", "name"]
        .iter()
        .find_map(|k| v.get(k).and_then(|x| x.as_str()))
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_audited() {
        for yes in [
            "/users",
            "/users/abc",
            "/users/abc/self",
            "/users/attr",
            "/users/abc/otp",
            "/clients/x/access_rules",
            "/roles",
            "/settings/reminders",
            "/settings/general",
            "/sessions",
            "/pam/hosts",
            "/providers/x",
            "/tos",
            "/backup",
        ] {
            assert!(audited(yes), "{yes}");
        }
        for no in [
            "/oidc/token",
            "/oidc/authorize",
            "/users/register",
            "/users/request_reset",
            "/users/abc/webauthn/auth/start",
            "/users/abc/webauthn/auth/finish",
            "/users/abc/mfa_token",
            "/users/abc/verification/email/start",
            "/settings/reminders/test",
            "/settings/smtp/test",
            "/settings/templates/x/preview",
            "/pam/login",
            "/pam/getent",
            "/providers/callback",
            "/providers/login",
            "/tos/accept",
            "/clients/validate_uri",
            "/events/read",
            "/events/test",
            "/pow",
            "",
        ] {
            assert!(!audited(no), "{no}");
        }
    }

    #[test]
    fn targets() {
        let t = |p: &str, param: Option<&str>| target_of(p, param.map(String::from));
        assert_eq!(
            t("/users/abc", Some("abc")),
            ("users".into(), Some("abc".into()))
        );
        assert_eq!(t("/roles", None), ("roles".into(), None));
        assert_eq!(t("/users/attr", None), ("users".into(), None));
        assert_eq!(
            t("/settings/reminders", None),
            ("settings".into(), Some("reminders".into()))
        );
        assert_eq!(t("/sessions", None), ("sessions".into(), None));
    }

    #[test]
    fn actions() {
        assert_eq!(action_of(&Method::POST, "/users"), "create");
        assert_eq!(action_of(&Method::POST, "/users/attr"), "create");
        assert_eq!(action_of(&Method::PUT, "/users/{id}"), "update");
        assert_eq!(action_of(&Method::PATCH, "/users/{id}"), "update");
        assert_eq!(action_of(&Method::DELETE, "/users/{id}"), "delete");
        assert_eq!(action_of(&Method::POST, "/users/{id}/otp"), "action");
        assert_eq!(action_of(&Method::PUT, "/clients/{id}/secret"), "action");
    }

    #[test]
    fn the_id_of_a_new_thing() {
        assert_eq!(
            id_from_body(br#"{"id":"abc","name":"x"}"#),
            Some("abc".into())
        );
        assert_eq!(id_from_body(br#"{"name":"x"}"#), Some("x".into()));
        assert_eq!(id_from_body(br#"{"a":1}"#), None);
        assert_eq!(id_from_body(b"not json"), None);
    }
}
