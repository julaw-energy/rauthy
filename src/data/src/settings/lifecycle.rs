//! Two settings of a user's life with the system, both set by an admin in the UI:
//!
//! - `new_users`: the roles and groups every new user gets,
//! - `reminders`: the reminder for users who have not logged in for a long time.
//!
//! They are read from memory like the other settings, so reading them costs nothing.

use super::{bad, live, reload, upsert};
use crate::entity::groups::Group;
use crate::entity::roles::Role;
use crate::entity::users::User;
use crate::message_templates::fill;
use rauthy_api_types::settings::{
    NewUsersSettings, NewUsersSettingsResponse, ReminderSettings, ReminderSettingsResponse,
};
use rauthy_error::ErrorResponse;

pub const KEY_NEW_USERS: &str = "new_users";
pub const KEY_REMINDERS: &str = "reminders";

pub const DEFAULT_REMINDER_DAYS: u32 = 30;
pub const DEFAULT_REMINDER_MAX: u8 = 3;
pub const DEFAULT_REMINDER_SUBJECT: &str = "We have not seen you for a while";
pub const DEFAULT_REMINDER_BODY: &str = "Hello {{name}},\n\n\
It has been {{days}} days since you last logged in to {{app_name}}. \
Your account is still there, and you can log in again any time:\n\n\
{{link}}\n\n\
If you do not need the account any more, you can ignore this message.";

const MAX_LISTED: usize = 50;

// ---- new users ---------------------------------------------------------------------------

/// A role nobody may hand out automatically: it would make every new user an admin
fn is_admin_role(name: &str) -> bool {
    name == "rauthy_admin" || name.starts_with("rauthy_admin:")
}

/// The roles and groups new users get right now
pub fn new_users() -> NewUsersSettings {
    live().new_users.clone().unwrap_or_default()
}

pub async fn new_users_view() -> Result<NewUsersSettingsResponse, ErrorResponse> {
    let mut available_roles: Vec<String> = Role::find_all()
        .await?
        .into_iter()
        .map(|r| r.name)
        .filter(|n| !is_admin_role(n))
        .collect();
    available_roles.sort();
    let mut available_groups: Vec<String> = Group::find_all()
        .await?
        .into_iter()
        .map(|g| g.name)
        .collect();
    available_groups.sort();

    Ok(NewUsersSettingsResponse {
        values: new_users(),
        available_roles,
        available_groups,
    })
}

pub async fn save_new_users(v: NewUsersSettings) -> Result<(), ErrorResponse> {
    if v.roles.len() > MAX_LISTED || v.groups.len() > MAX_LISTED {
        return Err(bad(format!(
            "At most {MAX_LISTED} roles and {MAX_LISTED} groups can be set"
        )));
    }
    if let Some(r) = v.roles.iter().find(|r| is_admin_role(r)) {
        return Err(bad(format!(
            "'{r}' cannot be a default role: every new user would become an admin"
        )));
    }

    let roles: Vec<String> = Role::find_all()
        .await?
        .into_iter()
        .map(|r| r.name)
        .collect();
    if let Some(r) = v.roles.iter().find(|r| !roles.contains(r)) {
        return Err(bad(format!("The role '{r}' does not exist")));
    }
    let groups: Vec<String> = Group::find_all()
        .await?
        .into_iter()
        .map(|g| g.name)
        .collect();
    if let Some(g) = v.groups.iter().find(|g| !groups.contains(g)) {
        return Err(bad(format!("The group '{g}' does not exist")));
    }

    let clean = NewUsersSettings {
        roles: dedup(v.roles),
        groups: dedup(v.groups),
    };
    upsert(KEY_NEW_USERS, &serde_json::to_string(&clean)?).await?;
    reload().await
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| seen.insert(s.clone()));
    v
}

/// Adds `add` to a comma separated list without repeating what is in it
fn merge_csv(current: &str, add: &[String]) -> String {
    let mut parts: Vec<&str> = current.split(',').filter(|s| !s.is_empty()).collect();
    for a in add {
        if !parts.contains(&a.as_str()) {
            parts.push(a);
        }
    }
    parts.join(",")
}

/// Gives a user that is about to be created the default roles and groups. Roles and groups that
/// were deleted since are skipped, they never make the registration fail.
pub async fn apply_new_user_defaults(user: &mut User) -> Result<(), ErrorResponse> {
    let d = new_users();
    if d.roles.is_empty() && d.groups.is_empty() {
        return Ok(());
    }

    if !d.roles.is_empty() {
        let existing = Role::sanitize(d.roles).await?;
        let existing: Vec<String> = existing.split(',').map(String::from).collect();
        user.roles = merge_csv(&user.roles, &existing);
    }
    if !d.groups.is_empty()
        && let Some(existing) = Group::sanitize(Some(d.groups)).await?
    {
        let existing: Vec<String> = existing.split(',').map(String::from).collect();
        let merged = merge_csv(user.groups.as_deref().unwrap_or_default(), &existing);
        user.groups = Some(merged).filter(|g| !g.is_empty());
    }
    Ok(())
}

// ---- reminders ---------------------------------------------------------------------------

pub fn default_reminders() -> ReminderSettings {
    ReminderSettings {
        // off until an admin turns it on: switching it on for a big user base mails everyone
        // who has been away, so it is a decision, not a surprise
        enabled: false,
        after_days: DEFAULT_REMINDER_DAYS,
        max_reminders: DEFAULT_REMINDER_MAX,
        subject: DEFAULT_REMINDER_SUBJECT.to_string(),
        body: DEFAULT_REMINDER_BODY.to_string(),
    }
}

/// The reminder settings in use right now
pub fn reminders() -> ReminderSettings {
    live().reminders.clone().unwrap_or_else(default_reminders)
}

pub async fn reminders_view() -> Result<ReminderSettingsResponse, ErrorResponse> {
    let values = reminders();
    let due_now = crate::entity::inactivity_reminders::InactivityReminders::count_due(&values)
        .await
        .unwrap_or_default();
    Ok(ReminderSettingsResponse { values, due_now })
}

const SUBJECT_VARS: &[&str] = &["name", "days", "app_name"];
const BODY_VARS: &[&str] = &["name", "days", "link", "app_name"];

/// The `{{name}}` placeholders in a text that are not in `allowed`
fn unknown_placeholders(text: &str, allowed: &[&str]) -> Vec<String> {
    let mut bad = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            bad.push(rest[start..].chars().take(12).collect());
            break;
        };
        let name = after[..end].trim();
        if !allowed.contains(&name) {
            bad.push(format!("{{{{{name}}}}}"));
        }
        rest = &after[end + 2..];
    }
    bad
}

pub async fn save_reminders(v: ReminderSettings) -> Result<(), ErrorResponse> {
    if !(1..=3650).contains(&v.after_days) {
        return Err(bad("The reminder time must be between 1 and 3650 days"));
    }
    if !(1..=10).contains(&v.max_reminders) {
        return Err(bad("A user can get between 1 and 10 reminders"));
    }
    let subject = v.subject.trim();
    let body = v.body.trim();
    if subject.is_empty() || subject.len() > 200 {
        return Err(bad("The subject must have 1 to 200 characters"));
    }
    if subject.contains('\n') || subject.contains('\r') {
        return Err(bad("The subject must be a single line"));
    }
    if body.is_empty() || body.len() > 5000 {
        return Err(bad("The text must have 1 to 5000 characters"));
    }
    for (what, text, allowed) in [
        ("subject", subject, SUBJECT_VARS),
        ("text", body, BODY_VARS),
    ] {
        let unknown = unknown_placeholders(text, allowed);
        if !unknown.is_empty() {
            return Err(bad(format!(
                "The {what} has placeholders that do not exist: {}. Available: {}",
                unknown.join(", "),
                allowed
                    .iter()
                    .map(|a| format!("{{{{{a}}}}}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    let clean = ReminderSettings {
        subject: subject.to_string(),
        body: body.to_string(),
        ..v
    };
    upsert(KEY_REMINDERS, &serde_json::to_string(&clean)?).await?;
    reload().await
}

/// The subject and the text of the reminder for one user
pub fn render_reminder(
    s: &ReminderSettings,
    name: &str,
    days: i64,
    link: &str,
    app_name: &str,
) -> (String, String) {
    let days = days.to_string();
    let subject = fill(
        &s.subject,
        &[("name", name), ("days", &days), ("app_name", app_name)],
    );
    let body = fill(
        &s.body,
        &[
            ("name", name),
            ("days", &days),
            ("link", link),
            ("app_name", app_name),
        ],
    );
    (subject, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_roles_cannot_be_defaults() {
        assert!(is_admin_role("rauthy_admin"));
        assert!(is_admin_role("rauthy_admin:sales"));
        assert!(!is_admin_role("admin"));
        assert!(!is_admin_role("user"));
        assert!(!is_admin_role("rauthy_admins"));
    }

    #[test]
    fn merging_lists_keeps_what_is_there() {
        let add = vec!["user".to_string(), "staff".to_string()];
        assert_eq!(merge_csv("", &add), "user,staff");
        assert_eq!(merge_csv("user", &add), "user,staff");
        assert_eq!(merge_csv("rauthy_admin", &add), "rauthy_admin,user,staff");
        assert_eq!(merge_csv("a,b", &[]), "a,b");
        assert_eq!(merge_csv("", &[]), "");
    }

    #[test]
    fn dedup_keeps_the_first() {
        let v = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        assert_eq!(dedup(v), ["a", "b"]);
    }

    #[test]
    fn placeholders_are_checked() {
        assert!(unknown_placeholders("Hi {{name}}, {{ days }} days", BODY_VARS).is_empty());
        assert_eq!(
            unknown_placeholders("Hi {{nam}}", BODY_VARS),
            vec!["{{nam}}".to_string()]
        );
        assert_eq!(
            unknown_placeholders("{{link}}", SUBJECT_VARS),
            vec!["{{link}}"]
        );
        assert_eq!(unknown_placeholders("open {{name", BODY_VARS).len(), 1);
        assert!(unknown_placeholders("no placeholders", BODY_VARS).is_empty());
    }

    #[test]
    fn the_default_text_only_uses_known_placeholders() {
        assert!(unknown_placeholders(DEFAULT_REMINDER_SUBJECT, SUBJECT_VARS).is_empty());
        assert!(unknown_placeholders(DEFAULT_REMINDER_BODY, BODY_VARS).is_empty());
    }

    #[test]
    fn the_reminder_text_is_filled() {
        let (subject, body) = render_reminder(
            &default_reminders(),
            "Jane",
            45,
            "https://id.example.com/auth/v1/account",
            "Julaw Accounts",
        );
        assert_eq!(subject, DEFAULT_REMINDER_SUBJECT);
        assert!(body.contains("Hello Jane,"));
        assert!(body.contains("45 days"));
        assert!(body.contains("Julaw Accounts"));
        assert!(body.contains("https://id.example.com/auth/v1/account"));
        assert!(!body.contains("{{"));
    }

    #[test]
    fn reminders_are_off_by_default_with_30_days() {
        let d = default_reminders();
        assert!(!d.enabled);
        assert_eq!(d.after_days, 30);
        assert_eq!(d.max_reminders, 3);
    }
}
