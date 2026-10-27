//! The reminder for users who have not logged in for a long time. The wording is set by an admin
//! under Config -> New users (see [`crate::settings::reminders`]).

use crate::email::custom::{build_custom_html_email, send_custom};
use crate::entity::users::User;
use crate::rauthy_config::RauthyConfig;
use crate::settings::{self, render_reminder};
use rauthy_api_types::settings::ReminderSettings;
use rauthy_error::ErrorResponse;

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The text as paragraphs, with the link clickable. Everything is escaped, an admin cannot put
/// markup into the mail.
fn text_to_html(text: &str, link: &str) -> String {
    let link_html = escape_html(link);
    let anchor = format!(r#"<a href="{link_html}">{link_html}</a>"#);
    text.split("\n\n")
        .map(|p| {
            let p = escape_html(p.trim()).replace('\n', "<br>");
            format!("<p>{}</p>", p.replace(&link_html, &anchor))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn send_inactivity_reminder(
    user: &User,
    days_inactive: i64,
    s: &ReminderSettings,
) -> Result<(), ErrorResponse> {
    let link = format!(
        "{}/auth/v1/account",
        RauthyConfig::get().pub_url_with_scheme
    );
    let (subject, text) = render_reminder(
        s,
        &user.email_recipient_name(),
        days_inactive,
        &link,
        &settings::app_name(),
    );
    let html = build_custom_html_email(&text_to_html(&text, &link)).await?;
    send_custom(user, &subject, Some(text), Some(html)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_becomes_escaped_paragraphs_with_a_clickable_link() {
        let link = "https://id.example.com/auth/v1/account";
        let html = text_to_html(
            &format!("Hello <b>Jane</b> & co,\n\nlog in: {link}\n\nBye\nTeam"),
            link,
        );
        assert!(html.contains("<p>Hello &lt;b&gt;Jane&lt;/b&gt; &amp; co,</p>"));
        assert!(html.contains(&format!(r#"<a href="{link}">{link}</a>"#)));
        assert!(html.contains("<p>Bye<br>Team</p>"));
        assert!(
            !html.contains("<b>"),
            "markup from the text is never passed on"
        );
    }
}
