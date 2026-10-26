//! Serves the documentation book (mdBook) from inside the binary at `/auth/v1/book/`.
//!
//! The book is built by `book/build-embed.sh` into `static/book/` before `cargo build`. Only the
//! gzip-compressed files are embedded, which keeps the binary about 2.5 MB bigger. If the folder
//! is empty the endpoint just answers with a hint, nothing breaks.
//!
//! It is only open to a logged-in session, unless `server.book_public` is set. The book describes
//! the configuration and internals, which is nothing the public needs.

use crate::ReqPrincipal;
use actix_web::http::header;
use actix_web::{HttpRequest, HttpResponse, get, web};
use rauthy_data::rauthy_config::RauthyConfig;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../static/book/"]
#[allow_missing = true]
struct BookAssets;

/// `None` when the request may see the book
fn deny(principal: &ReqPrincipal) -> Option<HttpResponse> {
    if RauthyConfig::get().vars.server.book_public || principal.validate_session_auth().is_ok() {
        None
    } else {
        Some(HttpResponse::Unauthorized().finish())
    }
}

#[get("/book")]
pub async fn get_book_redirect(principal: ReqPrincipal) -> HttpResponse {
    if let Some(denied) = deny(&principal) {
        return denied;
    }
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "/auth/v1/book/"))
        .finish()
}

#[get("/book/{tail:.*}")]
pub async fn get_book(
    req: HttpRequest,
    tail: web::Path<String>,
    principal: ReqPrincipal,
) -> HttpResponse {
    if let Some(denied) = deny(&principal) {
        return denied;
    }
    let mut path = tail.into_inner();
    if path.is_empty() || path.ends_with('/') {
        path.push_str("index.html");
    }

    // already compressed formats (images, fonts) are embedded as they are
    let gz = BookAssets::get(&format!("{path}.gz"));
    let (file, gzipped) = match (gz, BookAssets::get(&path)) {
        (Some(f), _) => (f, true),
        (None, Some(f)) => (f, false),
        (None, None) => {
            return if BookAssets::iter().next().is_none() {
                HttpResponse::NotFound()
                    .content_type("text/plain; charset=utf-8")
                    .body(
                        "The documentation is not built into this Rauthy binary. \
                    Run book/build-embed.sh before building.",
                    )
            } else {
                HttpResponse::NotFound().finish()
            };
        }
    };

    if gzipped {
        let accepts_gzip = req
            .headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("gzip"));
        if !accepts_gzip {
            return HttpResponse::NotAcceptable()
                .body("The documentation is served gzip-compressed, the client must accept it");
        }
    }

    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let mut resp = HttpResponse::Ok();
    resp.insert_header(("cache-control", "public, max-age=3600"))
        .insert_header((header::VARY, "accept-encoding"))
        .content_type(mime.as_ref());
    if gzipped {
        resp.insert_header((header::CONTENT_ENCODING, "gzip"));
    }
    resp.body(file.data.into_owned())
}
