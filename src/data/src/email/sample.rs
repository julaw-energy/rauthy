//! Renders the real email templates with the texts of a draft and sample values, for the
//! preview / test of the template editor. Nothing here touches what is sent to users.

use crate::email::email_registered_already::{
    EmailRegisteredAlreadyHtml, EmailRegisteredAlreadyTxt,
};
use crate::email::email_ts_prettify;
use crate::email::otp::{EmailOtpHtml, EmailOtpTxt};
use crate::email::password_reset::{EMailResetHtml, EmailResetTxt};
use crate::entity::users::User;
use crate::language::Language;
use crate::rauthy_config::{RauthyConfig, VarsTemplate};
use askama::Template;
use chrono::Utc;
use rauthy_error::{ErrorResponse, ErrorResponseType};

fn render_err(e: askama::Error) -> ErrorResponse {
    ErrorResponse::new(
        ErrorResponseType::BadRequest,
        format!("The template cannot be rendered: {e}"),
    )
}

/// `(subject, text, html)` of the email `id` for `user`, with the wording `t`
pub fn render(
    id: &str,
    lang: &Language,
    t: &VarsTemplate,
    user: &User,
    theme_vars: String,
) -> Result<(String, String, String), ErrorResponse> {
    let prefix = &RauthyConfig::get().vars.email.sub_prefix;
    let issuer = &RauthyConfig::get().issuer;
    let opt =
        |o: &Option<std::borrow::Cow<'static, str>>| o.as_deref().unwrap_or_default().to_string();
    let text_opt = t.text.clone().unwrap_or_default();
    let footer = t.footer.clone().unwrap_or_default();
    let subject = format!("{prefix} - {}", t.subject);
    let lang_str = user.language.as_str();

    match id {
        "password_new" | "password_reset" => {
            let is_new = id == "password_new";
            let link = format!("{issuer}users/{}/reset/sample?type=password_reset", user.id);
            let exp = email_ts_prettify(Utc::now().timestamp() + 1800, lang, None);
            let request_new = if is_new {
                String::new()
            } else {
                format!("{issuer}users/password_reset?email_hint={}", user.email)
            };
            let click_link = opt(&t.click_link);
            let validity = opt(&t.validity);
            let expires = opt(&t.expires);
            let button = opt(&t.button);
            let button_new = if is_new {
                String::new()
            } else {
                opt(&t.button_text_request_new)
            };

            let text = EmailResetTxt {
                email_sub_prefix: prefix,
                link: &link,
                exp: &exp,
                header: &t.header,
                click_link: &click_link,
                text: &text_opt,
                validity: &validity,
                expires: &expires,
                footer: &footer,
                link_request_new: &request_new,
            }
            .render()
            .map_err(render_err)?;
            let html = EMailResetHtml {
                lang: lang_str,
                theme_vars,
                email_sub_prefix: prefix,
                link: &link,
                exp: &exp,
                header: &t.header,
                click_link: &click_link,
                text: &text_opt,
                validity: &validity,
                expires: &expires,
                button_text: &button,
                footer: &footer,
                link_request_new: &request_new,
                button_text_request_new: &button_new,
            }
            .render()
            .map_err(render_err)?;
            Ok((subject, text, html))
        }
        "email_registered_already" => {
            let request_new = format!("{issuer}users/password_reset?email_hint={}", user.email);
            let button_new = opt(&t.button_text_request_new);
            let text = EmailRegisteredAlreadyTxt {
                email_sub_prefix: prefix,
                link_request_new: &request_new,
                header: &t.header,
                text: &text_opt,
                footer: &footer,
            }
            .render()
            .map_err(render_err)?;
            let html = EmailRegisteredAlreadyHtml {
                lang: lang_str,
                theme_vars,
                email_sub_prefix: prefix,
                link_request_new: &request_new,
                header: &t.header,
                text: &text_opt,
                footer: &footer,
                button_text_request_new: &button_new,
            }
            .render()
            .map_err(render_err)?;
            Ok((subject, text, html))
        }
        "email_otp" => {
            let text = EmailOtpTxt {
                email_sub_prefix: prefix,
                code: "123456",
                header: &t.header,
                text: &text_opt,
            }
            .render()
            .map_err(render_err)?;
            let html = EmailOtpHtml {
                lang: lang_str,
                theme_vars,
                email_sub_prefix: prefix,
                code: "123456",
                header: &t.header,
                text: &text_opt,
            }
            .render()
            .map_err(render_err)?;
            Ok((subject, text, html))
        }
        other => Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            format!("Unknown email template '{other}'"),
        )),
    }
}
