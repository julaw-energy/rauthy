use crate::ReqPrincipal;
use actix_web::http::header::{CONTENT_SECURITY_POLICY, HeaderValue};
use actix_web::{HttpResponse, get};
use rauthy_data::rauthy_config::RauthyConfig;
use rauthy_error::ErrorResponse;
use tracing::warn;

/// Pinned, so a CDN update can never change what an admin session loads.
const SCALAR_SCRIPT: &str =
    "https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.72.4/dist/browser/standalone.js";

/// A tiny HTML page that loads Scalar from a CDN and points it at `openapi.json`.
/// Nothing is bundled into the binary and nothing is kept in memory.
#[get("/docs/scalar")]
pub async fn get_scalar(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    scalar_page(&principal)
}

/// Also used by `swagger_ui::get_swagger_ui` to show Scalar on the `/docs/` root when enabled.
pub fn scalar_page(principal: &ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    let cfg = &RauthyConfig::get().vars.server;
    if !cfg.scalar_enable {
        warn!("Request for the Scalar UI but `scalar_enable` is set to `false`");
        return Ok(HttpResponse::NotFound().finish());
    }
    if !cfg.swagger_ui_public && principal.validate_session_auth().is_err() {
        warn!("Public request for the Scalar UI but `swagger_ui_public` is set to `false`");
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let csp = [
        "default-src 'none'",
        "font-src 'self' https://fonts.scalar.com",
        "img-src 'self' data:",
        "script-src 'self' 'unsafe-eval' https://cdn.jsdelivr.net",
        "style-src 'unsafe-inline'",
        "connect-src 'self'",
        "form-action 'none'",
        &format!(
            "frame-ancestors {}",
            RauthyConfig::get().pub_url_with_scheme
        ),
    ]
    .join("; ");

    let app_name = rauthy_data::settings::app_name()
        .replace('&', "&amp;")
        .replace('<', "&lt;");
    let body = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="referrer" content="no-referrer">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{app_name} API Reference</title>
  </head>
  <body>
    <script id="api-reference" data-url="openapi.json" data-configuration='{{"hideClientButton":false,"persistAuth":false,"withCredentials":true}}'></script>
    <script src="scalar-init.js"></script>
    <script src="{SCALAR_SCRIPT}"></script>
  </body>
</html>"#
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header((
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_str(&csp).unwrap(),
        ))
        .body(body))
}

/// Makes "Send request" work with the logged-in session, like the Swagger UI did: requests to this
/// origin carry the session cookie, and mutating ones get the CSRF token the UI keeps in the
/// browser. An API key is still possible, but not needed.
const SCALAR_INIT_JS: &str = r#"(function () {
    var orig = window.fetch.bind(window);
    function csrf() {
        try {
            var o = localStorage.getItem('sct') || '';
            return o.indexOf('o/') === 0 ? o.slice(41, o.length - 43) : o;
        } catch (e) {
            return '';
        }
    }
    window.fetch = function (input, init) {
        try {
            var req = input instanceof Request ? input : null;
            var url = new URL(req ? req.url : String(input), location.href);
            if (url.origin === location.origin) {
                init = init || {};
                if (!req || !init.credentials) init.credentials = 'include';
                var method = (init.method || (req && req.method) || 'GET').toUpperCase();
                if (method !== 'GET' && method !== 'HEAD' && method !== 'OPTIONS') {
                    var h = new Headers(init.headers || (req && req.headers) || {});
                    if (!h.has('x-csrf-token')) h.set('x-csrf-token', csrf());
                    init.headers = h;
                }
            }
        } catch (e) {}
        return orig(input, init);
    };
})();"#;

#[get("/docs/scalar-init.js")]
pub async fn get_scalar_init(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    let cfg = &RauthyConfig::get().vars.server;
    if !cfg.scalar_enable {
        return Ok(HttpResponse::NotFound().finish());
    }
    if !cfg.swagger_ui_public && principal.validate_session_auth().is_err() {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    Ok(HttpResponse::Ok()
        .content_type("application/javascript; charset=utf-8")
        .body(SCALAR_INIT_JS))
}
