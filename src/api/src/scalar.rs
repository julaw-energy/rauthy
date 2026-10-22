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
        "script-src 'unsafe-eval' https://cdn.jsdelivr.net",
        "style-src 'unsafe-inline'",
        "connect-src 'self'",
        "form-action 'none'",
        &format!(
            "frame-ancestors {}",
            RauthyConfig::get().pub_url_with_scheme
        ),
    ]
    .join("; ");

    let body = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="referrer" content="no-referrer">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Rauthy API Reference</title>
  </head>
  <body>
    <script id="api-reference" data-url="openapi.json" data-configuration='{{"hideClientButton":false,"persistAuth":false}}'></script>
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
