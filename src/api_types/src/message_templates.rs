use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

/// One editable text of a template
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct TemplateFieldInfo {
    pub name: String,
    pub label: String,
    pub required: bool,
    pub multiline: bool,
}

/// A `{{name}}` that can be used inside of the texts of a template
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct TemplatePlaceholderInfo {
    pub name: String,
    pub description: String,
    /// the text must contain it
    pub required: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct TemplateInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    /// `email` (styled HTML + text mail) or `message` (plain text for any channel)
    pub kind: String,
    pub fields: Vec<TemplateFieldInfo>,
    pub placeholders: Vec<TemplatePlaceholderInfo>,
    /// The languages that can be edited. `default` is the fallback for every language without
    /// its own text.
    pub languages: Vec<String>,
    /// The languages that were changed from the built-in text
    pub customized: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct TemplateDetail {
    pub id: String,
    pub lang: String,
    /// the texts in use right now
    pub values: BTreeMap<String, String>,
    /// the built-in texts, what a reset returns to
    pub defaults: BTreeMap<String, String>,
    pub customized: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TemplateSaveRequest {
    pub lang: String,
    /// every text of the template, an empty text unsets an optional one
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TemplateTestRequest {
    pub lang: String,
    /// the unsaved texts to try
    pub values: BTreeMap<String, String>,
    /// The channel to send a `message` template through, defaults to `email`. The test goes to
    /// the value of the logged in admin that the channel is tied to and fails if there is none.
    pub channel: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct TemplatePreviewResponse {
    pub subject: String,
    pub text: String,
    /// only for `email` templates
    pub html: Option<String>,
    /// things that are no error but worth knowing, like an SMS that needs several segments
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(debug_assertions, derive(Deserialize))]
pub struct TemplateTestResponse {
    pub message: String,
    pub warnings: Vec<String>,
}
