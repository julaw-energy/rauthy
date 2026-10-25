//! Texts of the messages Rauthy sends, editable at runtime.
//!
//! - The *layout* of an email (HTML / text skeleton) is compiled into the binary by askama and
//!   stays that way, so rendering costs what it always did.
//! - The *wording* (subject, header, intro, button texts, ...) of the four configurable emails
//!   and the text of the verification message for SMS / WhatsApp / any other channel live in
//!   the database (`settings`, keys `template.<id>.<lang>`).
//!
//! Everything is read from memory. A save is validated, written, and then only the template part
//! of the memory is swapped, so a send never waits for the database and never sees a half-saved
//! template. Other nodes pick the change up with the settings watcher.
//!
//! The memory swap leaks the previous email wording (a few KB per save), because the email i18n
//! code hands out `&'static` references. Templates are edited rarely, so this is on purpose.

use crate::email::sample;
use crate::entity::theme::ThemeCssFull;
use crate::entity::users::User;
use crate::language::Language;
use crate::rauthy_config::{RauthyConfig, VarsTemplate, VarsTemplates, VarsTemplatesLanguages};
use crate::settings;
use rauthy_api_types::message_templates::{
    TemplateDetail, TemplateFieldInfo, TemplateInfo, TemplatePlaceholderInfo,
    TemplatePreviewResponse,
};
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, Mutex, RwLock};
use strum::IntoEnumIterator;

pub const VERIFICATION: &str = "verification";
const KEY_PREFIX: &str = "template.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Email,
    Message,
}

pub struct Field {
    pub name: &'static str,
    pub label: &'static str,
    pub required: bool,
    pub multiline: bool,
}

pub struct Placeholder {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

pub struct TemplateDef {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub kind: Kind,
    pub fields: &'static [Field],
    pub placeholders: &'static [Placeholder],
}

const fn f(name: &'static str, label: &'static str, required: bool, multiline: bool) -> Field {
    Field {
        name,
        label,
        required,
        multiline,
    }
}

const APP_NAME: Placeholder = Placeholder {
    name: "app_name",
    description: "the name of this service",
    required: false,
};

static DEFS: &[TemplateDef] = &[
    TemplateDef {
        id: "password_new",
        title: "New account: set password",
        description: "Sent to a new user so they can set their first password.",
        kind: Kind::Email,
        fields: &[
            f("subject", "Subject", true, false),
            f("header", "Header", true, false),
            f("text", "Intro text", false, true),
            f("click_link", "Click-the-link line", true, false),
            f("validity", "Validity line", true, false),
            f("expires", "Expires label", true, false),
            f("button", "Button text", true, false),
            f("footer", "Footer", false, true),
        ],
        placeholders: &[APP_NAME],
    },
    TemplateDef {
        id: "password_reset",
        title: "Password reset",
        description: "Sent when a user asks to reset their password.",
        kind: Kind::Email,
        fields: &[
            f("subject", "Subject", true, false),
            f("header", "Header", true, false),
            f("text", "Intro text", false, true),
            f("click_link", "Click-the-link line", true, false),
            f("validity", "Validity line", true, false),
            f("expires", "Expires label", true, false),
            f("button", "Button text", true, false),
            f(
                "button_text_request_new",
                "Request-new-link button",
                false,
                false,
            ),
            f("footer", "Footer", false, true),
        ],
        placeholders: &[APP_NAME],
    },
    TemplateDef {
        id: "email_registered_already",
        title: "Email registered already",
        description: "Sent when someone registers with an address that already has an account.",
        kind: Kind::Email,
        fields: &[
            f("subject", "Subject", true, false),
            f("header", "Header", true, false),
            f("text", "Text", true, true),
            f(
                "button_text_request_new",
                "Request-new-link button",
                true,
                false,
            ),
            f("footer", "Footer", false, true),
        ],
        placeholders: &[APP_NAME],
    },
    TemplateDef {
        id: "email_otp",
        title: "Email one-time password",
        description: "Sent with the code for email based two-factor login.",
        kind: Kind::Email,
        fields: &[
            f("subject", "Subject", true, false),
            f("header", "Header", true, false),
            f("text", "Text", false, true),
        ],
        placeholders: &[APP_NAME],
    },
    TemplateDef {
        id: VERIFICATION,
        title: "Verification code message",
        description: "The message with the code that verifies an email address, phone number or \
            attribute. Plain text, sent through the channel the user picked.",
        kind: Kind::Message,
        fields: &[
            f("subject", "Subject (email channels only)", false, false),
            f("body", "Message", true, true),
        ],
        placeholders: &[
            Placeholder {
                name: "code",
                description: "the 6 digit code",
                required: true,
            },
            APP_NAME,
            Placeholder {
                name: "name",
                description: "the name of the user",
                required: false,
            },
            Placeholder {
                name: "minutes",
                description: "how long the code is valid",
                required: false,
            },
        ],
    },
];

pub fn defs() -> &'static [TemplateDef] {
    DEFS
}

pub fn def(id: &str) -> Option<&'static TemplateDef> {
    DEFS.iter().find(|d| d.id == id)
}

fn bad(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(ErrorResponseType::BadRequest, msg.into())
}

/// `default` (messages only) or the code of a language
pub fn lang_is_valid(d: &TemplateDef, lang: &str) -> bool {
    (d.kind == Kind::Message && lang == "default") || Language::iter().any(|l| l.as_str() == lang)
}

pub fn languages(d: &TemplateDef) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if d.kind == Kind::Message {
        v.push("default".into());
    }
    v.extend(Language::iter().map(|l| l.as_str().to_string()));
    v
}

// ---- placeholders ----------------------------------------------------------------------

/// Names of all `{{name}}` in the text. Fails on an unclosed or empty one.
fn placeholders_in(text: &str) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("a '{{' is never closed with '}}'".to_string());
        };
        let name = after[..end].trim();
        if name.is_empty() {
            return Err("an empty '{{}}'".to_string());
        }
        found.push(name.to_string());
        rest = &after[end + 2..];
    }
    Ok(found)
}

/// Replaces every `{{name}}` with its value. Unknown names are left as they are, [`validate`]
/// keeps them from being saved.
pub fn fill(text: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = after[..end].trim();
        match vars.iter().find(|(k, _)| *k == name) {
            Some((_, v)) => out.push_str(v),
            None => out.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

// ---- validation ------------------------------------------------------------------------

/// Checks the texts of a template. Returns warnings; an error means it must not be saved.
pub fn validate(
    d: &TemplateDef,
    values: &BTreeMap<String, String>,
) -> Result<Vec<String>, ErrorResponse> {
    validate_with(d, values, &settings::app_name())
}

fn validate_with(
    d: &TemplateDef,
    values: &BTreeMap<String, String>,
    app_name: &str,
) -> Result<Vec<String>, ErrorResponse> {
    let mut warnings = Vec::new();

    for k in values.keys() {
        if !d.fields.iter().any(|f| f.name == k) {
            return Err(bad(format!("'{k}' is not a text of this template")));
        }
    }

    let allowed: Vec<&str> = d.placeholders.iter().map(|p| p.name).collect();
    for field in d.fields {
        let v = values.get(field.name).map(String::as_str).unwrap_or("");
        let label = field.label;

        if v.trim().is_empty() {
            if field.required {
                return Err(bad(format!("'{label}' must not be empty")));
            }
            continue;
        }

        let max = if field.multiline { 2000 } else { 200 };
        if v.chars().count() > max {
            return Err(bad(format!("'{label}' is longer than {max} characters")));
        }
        if v.chars()
            .any(|c| c.is_control() && !(field.multiline && matches!(c, '\n' | '\r' | '\t')))
        {
            return Err(bad(format!(
                "'{label}' contains characters that are not allowed, like line breaks"
            )));
        }

        let used = placeholders_in(v).map_err(|e| bad(format!("'{label}' has {e}")))?;
        for u in &used {
            if !allowed.contains(&u.as_str()) {
                return Err(bad(format!(
                    "'{label}' uses {{{{{u}}}}}, which is not available here. Available: {}",
                    allowed
                        .iter()
                        .map(|a| format!("{{{{{a}}}}}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
    }

    // placeholders that the message cannot work without
    let main = if d.kind == Kind::Message { "body" } else { "" };
    if !main.is_empty() {
        let body = values.get(main).map(String::as_str).unwrap_or("");
        for p in d.placeholders.iter().filter(|p| p.required) {
            if !placeholders_in(body)
                .unwrap_or_default()
                .iter()
                .any(|u| u == p.name)
            {
                return Err(bad(format!(
                    "The message must contain {{{{{}}}}} ({})",
                    p.name, p.description
                )));
            }
        }
        if d.id == VERIFICATION {
            warnings.extend(sms_warnings(&fill(
                body,
                &[
                    ("code", "123456"),
                    ("app_name", app_name),
                    ("name", "Sample Name"),
                    ("minutes", "10"),
                ],
            )));
        }
    }

    Ok(warnings)
}

/// How many SMS segments a text needs, GSM-7 vs. UCS-2 depending on the characters
fn sms_warnings(text: &str) -> Vec<String> {
    let len = text.chars().count();
    let ascii = text.is_ascii();
    let (single, multi) = if ascii { (160, 153) } else { (70, 67) };
    if len <= single {
        return Vec::new();
    }
    let segments = len.div_ceil(multi);
    vec![format!(
        "The message has {len} characters and will be sent as {segments} SMS segments{}",
        if ascii {
            ""
        } else {
            " (it has characters that limit a segment to 70)"
        }
    )]
}

// ---- stored shape and memory ------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stored {
    values: BTreeMap<String, String>,
}

pub fn key(id: &str, lang: &str) -> String {
    format!("{KEY_PREFIX}{id}.{lang}")
}

struct State {
    /// `(id, lang)` -> texts
    overrides: HashMap<(String, String), BTreeMap<String, String>>,
    /// the email wording with all overrides applied, `None` while there are none
    emails: Option<&'static VarsTemplates>,
}

static STATE: LazyLock<RwLock<std::sync::Arc<State>>> = LazyLock::new(|| {
    RwLock::new(std::sync::Arc::new(State {
        overrides: HashMap::new(),
        emails: None,
    }))
});

/// signature of the input of the last build -> its result, so unchanged input never leaks again
static LAST_BUILD: Mutex<Option<(String, Option<&'static VarsTemplates>)>> = Mutex::new(None);

fn state() -> std::sync::Arc<State> {
    STATE.read().expect("templates lock poisoned").clone()
}

/// The email wording to use right now. This replaces reading it from the config directly.
pub fn email_templates() -> &'static VarsTemplates {
    state()
        .emails
        .unwrap_or(&RauthyConfig::get().vars.templates)
}

fn lang_slot<'a>(l: &'a mut VarsTemplatesLanguages, lang: &str) -> Option<&'a mut VarsTemplate> {
    Some(match lang {
        "de" => &mut l.de,
        "en" => &mut l.en,
        "es" => &mut l.es,
        "fr" => &mut l.fr,
        "ja" => &mut l.ja,
        "ko" => &mut l.ko,
        "nb" => &mut l.nb,
        "nl" => &mut l.nl,
        "pt" => &mut l.pt,
        "ru" => &mut l.ru,
        "uk" => &mut l.uk,
        "zhhans" => &mut l.zhhans,
        _ => return None,
    })
}

fn lang_ref<'a>(l: &'a VarsTemplatesLanguages, lang: &str) -> Option<&'a VarsTemplate> {
    Some(match lang {
        "de" => &l.de,
        "en" => &l.en,
        "es" => &l.es,
        "fr" => &l.fr,
        "ja" => &l.ja,
        "ko" => &l.ko,
        "nb" => &l.nb,
        "nl" => &l.nl,
        "pt" => &l.pt,
        "ru" => &l.ru,
        "uk" => &l.uk,
        "zhhans" => &l.zhhans,
        _ => return None,
    })
}

fn email_slot<'a>(t: &'a mut VarsTemplates, id: &str) -> Option<&'a mut VarsTemplatesLanguages> {
    Some(match id {
        "password_new" => &mut t.password_new,
        "password_reset" => &mut t.password_reset,
        "email_registered_already" => &mut t.email_registered_already,
        "email_otp" => &mut t.email_otp,
        _ => return None,
    })
}

fn email_ref<'a>(t: &'a VarsTemplates, id: &str) -> Option<&'a VarsTemplatesLanguages> {
    Some(match id {
        "password_new" => &t.password_new,
        "password_reset" => &t.password_reset,
        "email_registered_already" => &t.email_registered_already,
        "email_otp" => &t.email_otp,
        _ => return None,
    })
}

/// `{{app_name}}` -> the name
fn subst(s: &str, app_name: &str) -> String {
    fill(s, &[("app_name", app_name)])
}

/// The texts of a draft applied on top of a base wording
fn apply_values(
    d: &TemplateDef,
    base: &VarsTemplate,
    values: &BTreeMap<String, String>,
    app_name: &str,
) -> VarsTemplate {
    let mut t = base.clone();
    for field in d.fields {
        let Some(v) = values.get(field.name) else {
            continue;
        };
        let v = subst(v.trim(), app_name);
        let required = field.required;
        let opt_cow = |v: &str| (!v.is_empty()).then(|| Cow::Owned(v.to_string()));
        match field.name {
            "subject" => t.subject = Cow::Owned(v),
            "header" => t.header = Cow::Owned(v),
            "text" => t.text = (!v.is_empty()).then_some(v),
            "footer" => t.footer = (!v.is_empty()).then_some(v),
            "click_link" if required || !v.is_empty() => t.click_link = opt_cow(&v),
            "validity" if required || !v.is_empty() => t.validity = opt_cow(&v),
            "expires" if required || !v.is_empty() => t.expires = opt_cow(&v),
            "button" if required || !v.is_empty() => t.button = opt_cow(&v),
            "button_text_request_new" => t.button_text_request_new = opt_cow(&v),
            _ => {}
        }
    }
    t
}

/// Rebuilds the in-memory templates from the `template.*` rows of the settings. Called by every
/// settings reload.
pub(crate) fn apply(rows: &[(String, String)]) {
    let mut overrides: HashMap<(String, String), BTreeMap<String, String>> = HashMap::new();
    for (k, v) in rows {
        let Some(rest) = k.strip_prefix(KEY_PREFIX) else {
            continue;
        };
        let Some((id, lang)) = rest.split_once('.') else {
            continue;
        };
        let Some(d) = def(id) else {
            continue;
        };
        if !lang_is_valid(d, lang) {
            continue;
        }
        match serde_json::from_str::<Stored>(v) {
            Ok(s) => {
                overrides.insert((id.to_string(), lang.to_string()), s.values);
            }
            Err(err) => tracing::error!(?err, "Invalid template '{k}' in the DB"),
        }
    }

    let app_name = settings::app_name();
    let has_email = overrides
        .keys()
        .any(|(id, _)| def(id).is_some_and(|d| d.kind == Kind::Email));

    let emails = if has_email {
        let mut sig_parts: Vec<String> = overrides
            .iter()
            .filter(|((id, _), _)| def(id).is_some_and(|d| d.kind == Kind::Email))
            .map(|((id, lang), v)| format!("{id}.{lang}={v:?}"))
            .collect();
        sig_parts.sort();
        let sig = format!("{app_name}|{}", sig_parts.join("|"));

        let mut last = LAST_BUILD.lock().expect("templates build lock poisoned");
        match last.as_ref().filter(|(s, _)| *s == sig) {
            Some((_, built)) => *built,
            None => {
                let mut t = RauthyConfig::get().vars.templates.clone();
                for ((id, lang), values) in &overrides {
                    let Some(d) = def(id).filter(|d| d.kind == Kind::Email) else {
                        continue;
                    };
                    let Some(slot) = email_slot(&mut t, id).and_then(|l| lang_slot(l, lang)) else {
                        continue;
                    };
                    *slot = apply_values(d, slot, values, &app_name);
                }
                let built: &'static VarsTemplates = Box::leak(Box::new(t));
                *last = Some((sig, Some(built)));
                Some(built)
            }
        }
    } else {
        None
    };

    *STATE.write().expect("templates lock poisoned") =
        std::sync::Arc::new(State { overrides, emails });
}

// ---- reading ---------------------------------------------------------------------------

fn wording_to_values(d: &TemplateDef, t: &VarsTemplate) -> BTreeMap<String, String> {
    d.fields
        .iter()
        .map(|field| {
            let v = match field.name {
                "subject" => t.subject.to_string(),
                "header" => t.header.to_string(),
                "text" => t.text.clone().unwrap_or_default(),
                "footer" => t.footer.clone().unwrap_or_default(),
                "click_link" => t.click_link.as_deref().unwrap_or_default().to_string(),
                "validity" => t.validity.as_deref().unwrap_or_default().to_string(),
                "expires" => t.expires.as_deref().unwrap_or_default().to_string(),
                "button" => t.button.as_deref().unwrap_or_default().to_string(),
                "button_text_request_new" => t
                    .button_text_request_new
                    .as_deref()
                    .unwrap_or_default()
                    .to_string(),
                _ => String::new(),
            };
            (field.name.to_string(), v)
        })
        .collect()
}

fn builtin_message(id: &str) -> BTreeMap<String, String> {
    debug_assert_eq!(id, VERIFICATION);
    BTreeMap::from([
        ("subject".to_string(), "Verification code".to_string()),
        (
            "body".to_string(),
            "Your {{app_name}} verification code is {{code}}. It is valid for {{minutes}} minutes."
                .to_string(),
        ),
    ])
}

/// What a reset returns to: the config file / built-in wording
pub fn defaults(d: &TemplateDef, lang: &str) -> BTreeMap<String, String> {
    match d.kind {
        Kind::Message => builtin_message(d.id),
        Kind::Email => email_ref(&RauthyConfig::get().vars.templates, d.id)
            .and_then(|l| lang_ref(l, lang))
            .map(|t| wording_to_values(d, t))
            .unwrap_or_default(),
    }
}

pub fn detail(d: &TemplateDef, lang: &str) -> TemplateDetail {
    let st = state();
    let over = st.overrides.get(&(d.id.to_string(), lang.to_string()));
    let mut values = defaults(d, lang);
    if let Some(o) = over {
        for (k, v) in o {
            values.insert(k.clone(), v.clone());
        }
    }
    TemplateDetail {
        id: d.id.to_string(),
        lang: lang.to_string(),
        values,
        defaults: defaults(d, lang),
        customized: over.is_some(),
    }
}

pub fn info(d: &TemplateDef) -> TemplateInfo {
    let st = state();
    let mut customized: Vec<String> = st
        .overrides
        .keys()
        .filter(|(id, _)| id == d.id)
        .map(|(_, l)| l.clone())
        .collect();
    customized.sort();
    TemplateInfo {
        id: d.id.to_string(),
        title: d.title.to_string(),
        description: d.description.to_string(),
        kind: match d.kind {
            Kind::Email => "email",
            Kind::Message => "message",
        }
        .to_string(),
        fields: d
            .fields
            .iter()
            .map(|f| TemplateFieldInfo {
                name: f.name.to_string(),
                label: f.label.to_string(),
                required: f.required,
                multiline: f.multiline,
            })
            .collect(),
        placeholders: d
            .placeholders
            .iter()
            .map(|p| TemplatePlaceholderInfo {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect(),
        languages: languages(d),
        customized,
    }
}

/// The verification message for a user: `(subject, body)`
pub fn verification(lang: &str, code: &str, name: &str) -> (String, String) {
    let st = state();
    let d = def(VERIFICATION).expect("verification template");
    let values = st
        .overrides
        .get(&(VERIFICATION.to_string(), lang.to_string()))
        .or_else(|| {
            st.overrides
                .get(&(VERIFICATION.to_string(), "default".to_string()))
        })
        .cloned()
        .unwrap_or_else(|| defaults(d, "default"));

    let app_name = settings::app_name();
    let vars = [
        ("code", code),
        ("app_name", app_name.as_str()),
        ("name", name),
        ("minutes", "10"),
    ];
    let get = |k: &str| values.get(k).map(String::as_str).unwrap_or_default();
    (fill(get("subject"), &vars), fill(get("body"), &vars))
}

// ---- preview ---------------------------------------------------------------------------

/// Renders a draft with sample values for `user`. Nothing is saved or sent.
pub async fn preview(
    d: &TemplateDef,
    lang: &str,
    values: &BTreeMap<String, String>,
    user: &User,
) -> Result<TemplatePreviewResponse, ErrorResponse> {
    if !lang_is_valid(d, lang) {
        return Err(bad("Unknown language"));
    }
    let warnings = validate(d, values)?;

    match d.kind {
        Kind::Message => {
            let app_name = settings::app_name();
            let name = user.email_recipient_name();
            let vars = [
                ("code", "123456"),
                ("app_name", app_name.as_str()),
                ("name", name.as_str()),
                ("minutes", "10"),
            ];
            let get = |k: &str| values.get(k).map(String::as_str).unwrap_or_default();
            Ok(TemplatePreviewResponse {
                subject: fill(get("subject"), &vars),
                text: fill(get("body"), &vars),
                html: None,
                warnings,
            })
        }
        Kind::Email => {
            let base = email_ref(&RauthyConfig::get().vars.templates, d.id)
                .and_then(|l| lang_ref(l, lang))
                .ok_or_else(|| bad("Unknown language"))?;
            let tpl = apply_values(d, base, values, &settings::app_name());
            let language = Language::iter()
                .find(|l| l.as_str() == lang)
                .unwrap_or_default();
            let theme = ThemeCssFull::find_theme_variables_email()
                .await
                .unwrap_or_default();
            let (subject, text, html) = sample::render(d.id, &language, &tpl, user, theme)?;
            Ok(TemplatePreviewResponse {
                subject,
                text,
                html: Some(html),
                warnings,
            })
        }
    }
}

// ---- saving ----------------------------------------------------------------------------

/// Validates and stores the texts, then swaps the template part of the memory.
pub async fn save(
    d: &TemplateDef,
    lang: &str,
    values: BTreeMap<String, String>,
) -> Result<Vec<String>, ErrorResponse> {
    if !lang_is_valid(d, lang) {
        return Err(bad("Unknown language"));
    }
    let warnings = validate(d, &values)?;

    // trimmed, so a text of only spaces counts as unset
    let values: BTreeMap<String, String> = values
        .into_iter()
        .map(|(k, v)| (k, v.trim().to_string()))
        .collect();

    // a draft that equals the built-in text is not an override
    if values == defaults(d, lang) {
        reset(d, lang).await?;
        return Ok(warnings);
    }

    settings::upsert_raw(
        &key(d.id, lang),
        &serde_json::to_string(&Stored { values }).map_err(|e| bad(e.to_string()))?,
    )
    .await?;
    settings::reload().await?;
    Ok(warnings)
}

/// Removes the override, the built-in text is used again
pub async fn reset(d: &TemplateDef, lang: &str) -> Result<(), ErrorResponse> {
    if !lang_is_valid(d, lang) {
        return Err(bad("Unknown language"));
    }
    settings::delete_raw(&key(d.id, lang)).await?;
    settings::reload().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_known_and_keeps_unknown() {
        assert_eq!(
            fill(
                "Hi {{ name }}, {{code}} {{nope}}",
                &[("name", "A"), ("code", "1")]
            ),
            "Hi A, 1 {{nope}}"
        );
        assert_eq!(fill("open {{", &[]), "open {{");
    }

    #[test]
    fn placeholder_scan() {
        assert_eq!(placeholders_in("a {{x}} b {{ y }}").unwrap(), ["x", "y"]);
        assert!(placeholders_in("a {{x").is_err());
        assert!(placeholders_in("a {{}}").is_err());
    }

    #[test]
    fn verification_needs_code() {
        let d = def(VERIFICATION).unwrap();
        let ok = BTreeMap::from([("body".to_string(), "Code {{code}}".to_string())]);
        assert!(validate_with(d, &ok, "App").is_ok());

        let no_code = BTreeMap::from([("body".to_string(), "Hello".to_string())]);
        assert!(validate_with(d, &no_code, "App").is_err());

        let unknown = BTreeMap::from([("body".to_string(), "{{code}} {{secret}}".to_string())]);
        assert!(validate_with(d, &unknown, "App").is_err());

        let empty = BTreeMap::from([("body".to_string(), "  ".to_string())]);
        assert!(validate_with(d, &empty, "App").is_err());
    }

    #[test]
    fn sms_segments_warn() {
        assert!(sms_warnings(&"a".repeat(160)).is_empty());
        assert_eq!(sms_warnings(&"a".repeat(161)).len(), 1);
        assert_eq!(sms_warnings(&"é".repeat(71)).len(), 1);
    }

    #[test]
    fn email_fields_reject_line_breaks_in_single_line() {
        let d = def("email_otp").unwrap();
        let v = BTreeMap::from([
            ("subject".to_string(), "Hi\nthere".to_string()),
            ("header".to_string(), "H".to_string()),
        ]);
        assert!(validate_with(d, &v, "App").is_err());
    }
}
