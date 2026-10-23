use crate::channels::{
    ChannelDriver, ChannelMessage, ChannelRecipient, MessagePurpose, send as channel_send,
};
use crate::database::DB;
use crate::email;
use crate::entity::users::User;
use crate::entity::users_values::UserValues;
use crate::rauthy_config::RauthyConfig;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::email_jobs::{
    EmailJobFilterType, EmailJobRequest, EmailJobResponse, EmailTargets, MessageChannel,
};
use rauthy_common::{is_hiqlite, markdown};
use rauthy_error::{ErrorResponse, ErrorResponseType};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info, trace, warn};

#[derive(Debug, FromRow)]
pub struct EmailJob {
    pub id: i64,
    #[column(parse)]
    pub channel: MessageChannel,
    pub scheduled: Option<i64>,
    #[column(from_i64)]
    pub status: EmailJobStatus,
    pub updated: i64,
    pub last_user_ts: Option<String>,
    #[column(parse)]
    pub filter: EmailJobFilter,
    #[column(parse)]
    pub content_type: EmailContentType,
    pub subject: String,
    pub body: String,
}

impl From<tokio_postgres::Row> for EmailJob {
    fn from(row: tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            channel: row.get::<_, String>("channel").parse().unwrap_or_default(),
            scheduled: row.get("scheduled"),
            status: EmailJobStatus::from(row.get::<_, i16>("status") as i64),
            updated: row.get("updated"),
            last_user_ts: row.get("last_user_ts"),
            filter: row.get::<_, String>("filter").parse().unwrap(),
            content_type: row.get::<_, String>("content_type").parse().unwrap(),
            subject: row.get("subject"),
            body: row.get("body"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EmailContentType {
    Text,
    Markdown,
    Html,
}

impl FromStr for EmailContentType {
    type Err = ErrorResponse;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let slf = match s {
            "markdown" => Self::Markdown,
            "html" => Self::Html,
            _ => Self::Text,
        };
        Ok(slf)
    }
}

impl Display for EmailContentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailContentType::Text => write!(f, "text"),
            EmailContentType::Markdown => write!(f, "markdown"),
            EmailContentType::Html => write!(f, "html"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EmailJobFilter {
    None,
    InGroup(String),
    NotInGroup(String),
    HasRole(String),
    HasNotRole(String),
    /// Multi-select filter, stored as `targets_{json}` in the same `filter` column
    Targets(EmailTargets),
}

impl EmailJobFilter {
    /// Returns `true` if the user should receive the email of a job with this filter.
    fn matches(&self, user: &User) -> bool {
        match self {
            EmailJobFilter::None => true,
            EmailJobFilter::InGroup(s) => user.get_groups().contains(s),
            EmailJobFilter::NotInGroup(s) => !user.get_groups().contains(s),
            EmailJobFilter::HasRole(s) => user.get_roles().contains(s),
            EmailJobFilter::HasNotRole(s) => !user.get_roles().contains(s),
            EmailJobFilter::Targets(t) => {
                let roles = user.get_roles();
                let groups = user.get_groups();
                if t.not_roles.iter().any(|r| roles.contains(r))
                    || t.not_groups.iter().any(|g| groups.contains(g))
                {
                    return false;
                }
                !t.has_includes()
                    || t.users.contains(&user.id)
                    || t.roles.iter().any(|r| roles.contains(r))
                    || t.groups.iter().any(|g| groups.contains(g))
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EmailJobStatus {
    Open,
    Finished,
    Canceled,
}

impl EmailJobStatus {
    pub fn value(&self) -> i16 {
        match self {
            EmailJobStatus::Open => 0,
            EmailJobStatus::Finished => 1,
            EmailJobStatus::Canceled => 2,
        }
    }
}

impl From<i64> for EmailJobStatus {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::Open,
            1 => Self::Finished,
            2 => Self::Canceled,
            i => {
                panic!("Invalid EmailJobStatus in DB: {i}")
            }
        }
    }
}

impl FromStr for EmailJobFilter {
    type Err = ErrorResponse;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let slf = if let Some(group) = s.strip_prefix("group_") {
            Self::InGroup(group.to_string())
        } else if let Some(group) = s.strip_prefix("not_group_") {
            Self::NotInGroup(group.to_string())
        } else if let Some(role) = s.strip_prefix("role_") {
            Self::HasRole(role.to_string())
        } else if let Some(role) = s.strip_prefix("not_role_") {
            Self::HasNotRole(role.to_string())
        } else if let Some(json) = s.strip_prefix("targets_") {
            match serde_json::from_str::<EmailTargets>(json) {
                Ok(t) => Self::Targets(t),
                Err(err) => {
                    error!(?err, "Invalid EmailJobFilter targets in DB");
                    Self::None
                }
            }
        } else if s != "none" {
            error!("Invalid EmailJobFilter value in DB: {s}");
            Self::None
        } else {
            Self::None
        };

        Ok(slf)
    }
}

impl Display for EmailJobFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailJobFilter::None => {
                write!(f, "none")
            }
            EmailJobFilter::InGroup(s) => {
                write!(f, "group_{s}")
            }
            EmailJobFilter::NotInGroup(s) => {
                write!(f, "not_group_{s}")
            }
            EmailJobFilter::HasRole(s) => {
                write!(f, "role_{s}")
            }
            EmailJobFilter::HasNotRole(s) => {
                write!(f, "not_role_{s}")
            }
            EmailJobFilter::Targets(t) => {
                write!(
                    f,
                    "targets_{}",
                    serde_json::to_string(t).unwrap_or_default()
                )
            }
        }
    }
}

impl EmailJob {
    pub async fn cancel(&mut self) -> Result<(), ErrorResponse> {
        let sql = "UPDATE email_jobs SET status = $1 WHERE id = $2";

        let status = EmailJobStatus::Canceled.value();
        if is_hiqlite() {
            DB::hql().execute(sql, params!(status, self.id)).await?;
        } else {
            DB::pg_execute(sql, &[&status, &self.id]).await?;
        }
        self.status = EmailJobStatus::Canceled;

        Ok(())
    }

    /// Expects body to be already sanitized if `is_html`!
    pub async fn insert(payload: EmailJobRequest) -> Result<Self, ErrorResponse> {
        let now = Utc::now().timestamp();
        let status = EmailJobStatus::Open.value();
        let filter = if let Some(t) = payload.targets.filter(|t| !t.is_empty()) {
            EmailJobFilter::Targets(t)
        } else if let Some(value) = payload.filter_value {
            match payload.filter_type {
                EmailJobFilterType::None => EmailJobFilter::None,
                EmailJobFilterType::InGroup => EmailJobFilter::InGroup(value),
                EmailJobFilterType::NotInGroup => EmailJobFilter::NotInGroup(value),
                EmailJobFilterType::HasRole => EmailJobFilter::HasRole(value),
                EmailJobFilterType::HasNotRole => EmailJobFilter::HasNotRole(value),
            }
        } else {
            EmailJobFilter::None
        }
        .to_string();
        let content_type = EmailContentType::from(payload.content_type).to_string();
        let channel = payload.channel.as_str().to_string();

        let sql = r#"
INSERT INTO email_jobs (id, scheduled, status, updated, filter, content_type, subject, body, channel)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
RETURNING *"#;

        let slf = if is_hiqlite() {
            DB::hql()
                .execute_returning_map_one(
                    sql,
                    params!(
                        now,
                        payload.scheduled,
                        status,
                        now,
                        filter,
                        content_type,
                        payload.subject,
                        payload.body,
                        channel
                    ),
                )
                .await?
        } else {
            DB::pg_query_one(
                sql,
                &[
                    &now,
                    &payload.scheduled,
                    &status,
                    &now,
                    &filter,
                    &content_type,
                    &payload.subject,
                    &payload.body,
                    &channel,
                ],
            )
            .await?
        };

        Ok(slf)
    }

    pub async fn find(id: i64) -> Result<Self, ErrorResponse> {
        let sql = "SELECT * FROM email_jobs WHERE id = $1";

        let slf = if is_hiqlite() {
            DB::hql().query_map_one(sql, params!(id)).await?
        } else {
            DB::pg_query_one(sql, &[&id]).await?
        };

        Ok(slf)
    }

    pub async fn find_all() -> Result<Vec<Self>, ErrorResponse> {
        let sql = "SELECT * FROM email_jobs ORDER BY id DESC";

        let res = if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(sql, &[], 0).await?
        };

        Ok(res)
    }

    /// Finds jobs that have not been updated in a long time, or are scheduled for "now".
    ///
    /// In HA deployments, only the current leader should look for orphaned jobs and pick them
    /// up to avoid duplicate emails because of possible race conditions.
    pub async fn find_orphaned_or_schedule() -> Result<Vec<Self>, ErrorResponse> {
        let sql = r#"
SELECT * FROM email_jobs
WHERE status = 0 AND updated < $2 AND (scheduled IS NULL OR scheduled <= $1)"#;

        let now = Utc::now().timestamp();
        let threshold = now
            - RauthyConfig::get()
                .vars
                .email
                .jobs
                .orphaned_pickup
                .as_secs() as i64;

        let res = if is_hiqlite() {
            DB::hql().query_map(sql, params!(now, threshold)).await?
        } else {
            DB::pg_query(sql, &[&now, &threshold], 0).await?
        };

        Ok(res)
    }

    /// Updates:
    /// - scheduled
    /// - status
    /// - updated
    /// - last_user_ts
    pub async fn save(&mut self) -> Result<(), ErrorResponse> {
        let sql = r#"
UPDATE email_jobs
SET scheduled = $1, status = $2, updated = $3, last_user_ts = $4
WHERE id = $5"#;

        self.updated = Utc::now().timestamp();
        let status = self.status.value();
        if is_hiqlite() {
            DB::hql()
                .execute(
                    sql,
                    params!(
                        self.scheduled,
                        status,
                        self.updated,
                        &self.last_user_ts,
                        self.id
                    ),
                )
                .await?;
        } else {
            DB::pg_execute(
                sql,
                &[
                    &self.scheduled,
                    &status,
                    &self.updated,
                    &self.last_user_ts,
                    &self.id,
                ],
            )
            .await?;
        }

        Ok(())
    }
}

impl EmailJob {
    async fn render_html_content(&self) -> Result<Option<String>, ErrorResponse> {
        let body = match self.content_type {
            EmailContentType::Text => {
                return Ok(None);
            }
            EmailContentType::Markdown => {
                Cow::from(markdown::render_sanitized_markdown(&self.body))
            }
            EmailContentType::Html => Cow::from(self.body.to_string()),
        };

        Ok(Some(email::custom::build_custom_html_email(&body).await?))
    }

    pub fn spawn_task(self) {
        info!("Spawning EmailJob background task {}", self.id);

        tokio::task::spawn(async move {
            if let Err(err) = self.task_execute().await {
                error!("Error during EmailJob Task: {}", err.message);
            }
        });
    }

    #[inline]
    async fn task_execute(mut self) -> Result<(), ErrorResponse> {
        let (batch_size, delay) = {
            let vars = &RauthyConfig::get().vars.email.jobs;
            (vars.batch_size, vars.batch_delay)
        };
        let mut delay_inner_ms = 10;

        let (text, html) = match self.content_type {
            EmailContentType::Text => (Some(self.body.clone()), None),
            EmailContentType::Markdown => {
                (Some(self.body.clone()), self.render_html_content().await?)
            }
            EmailContentType::Html => (None, self.render_html_content().await?),
        };

        // everything except the built-in SMTP mailer goes through the configured agent driver
        let via_smtp = self.channel == MessageChannel::Email
            && crate::settings::channel(self.channel).driver == ChannelDriver::Smtp;
        let mut consecutive_failures = 0;

        let mut cancel_check_ts = Utc::now().timestamp();

        loop {
            {
                let now = Utc::now().timestamp();
                if now + 30 > cancel_check_ts {
                    let slf = Self::find(self.id).await?;
                    if slf.status == EmailJobStatus::Canceled {
                        warn!("EmailJob Task has been canceled externally - exiting");
                        return Ok(());
                    }
                    cancel_check_ts = now;
                }
            }

            if self.status != EmailJobStatus::Open {
                info!(
                    "EmailJob Task {} finished with status {:?}",
                    self.id, self.status
                );
                return Ok(());
            }

            let users = {
                let (email, ts) = self.last_user_ts();
                User::find_batch(email, ts, batch_size).await?
            };
            if users.is_empty() {
                self.status = EmailJobStatus::Finished;
                self.save().await?;
                continue;
            }

            for user in users {
                if !self.filter.matches(&user) {
                    self.set_last_user_ts(&user.email, user.created_at);
                    continue;
                }

                if !via_smtp {
                    match self.deliver_via_agent(&user, &text, &html).await {
                        Ok(_) => consecutive_failures = 0,
                        Err(err) => {
                            // one bad number / address must not kill the whole job, but a dead
                            // agent should
                            consecutive_failures += 1;
                            error!(
                                "EmailJob {}: delivery to user {} failed: {}",
                                self.id, user.id, err.message
                            );
                            if consecutive_failures >= 5 {
                                self.set_last_user_ts(&user.email, user.created_at);
                                self.save().await?;
                                return Err(ErrorResponse::new(
                                    ErrorResponseType::Connection,
                                    "5 deliveries in a row failed - exiting job",
                                ));
                            }
                        }
                    }
                    self.set_last_user_ts(&user.email, user.created_at);
                    tokio::time::sleep(Duration::from_millis(delay_inner_ms)).await;
                    continue;
                }

                // catch a send timeout and increase delay timing
                for i in 0..5 {
                    match email::custom::send_custom(
                        &user,
                        &self.subject,
                        text.clone(),
                        html.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            trace!(
                                "E-Mail to {} sent to Mailer-Task (internally) successfully",
                                user.email
                            );
                            self.set_last_user_ts(&user.email, user.created_at);
                            break;
                        }
                        Err(err) => {
                            if i >= 4 {
                                self.save().await?;
                                return Err(ErrorResponse::new(
                                    ErrorResponseType::Timeout,
                                    "Reached E-Mail send timeout - retries exceeded - \
                                    exiting job",
                                ));
                            }

                            let delay_new = delay_inner_ms * 3;
                            error!(
                                ?err,
                                "Reached E-Mail send timeout after {i} retries, increasing batch \
                                delay_inner_ms from {delay_inner_ms} to {delay_new}"
                            );
                            delay_inner_ms = delay_new;

                            tokio::time::sleep(Duration::from_millis(delay_inner_ms)).await;
                        }
                    }
                }

                // Email sending happens on a dedicated thread and if we would not sleep at least
                // a short amount of time inside the batch, the channel could get filled up pretty
                // quickly with slow servers.
                tokio::time::sleep(Duration::from_millis(delay_inner_ms)).await;
            }

            self.save().await?;
            tokio::time::sleep(delay).await;
        }
    }

    /// Delivers to one user through the agent driver of the job's channel.
    /// Users without the needed contact value (e.g. no phone number) are skipped.
    async fn deliver_via_agent(
        &self,
        user: &User,
        text: &Option<String>,
        html: &Option<String>,
    ) -> Result<(), ErrorResponse> {
        let to = match self.channel {
            MessageChannel::Email => user.email.clone(),
            MessageChannel::Sms | MessageChannel::Whatsapp => {
                match UserValues::find(&user.id).await?.and_then(|v| v.phone) {
                    Some(p) if !p.is_empty() => p,
                    _ => {
                        trace!("EmailJob {}: user {} has no phone number", self.id, user.id);
                        return Ok(());
                    }
                }
            }
        };

        let msg = ChannelMessage::new(
            self.channel,
            MessagePurpose::Admin,
            ChannelRecipient {
                user_id: user.id.clone(),
                name: user.email_recipient_name(),
                to,
            },
            (self.channel == MessageChannel::Email).then(|| self.subject.clone()),
            text.clone().unwrap_or_default(),
            html.clone(),
            Some(format!("emailjob-{}-{}", self.id, user.id)),
        );
        channel_send(self.channel, &msg).await.map(|_| ())
    }

    #[inline]
    fn last_user_ts(&self) -> (&str, i64) {
        if let Some(last_user_ts) = &self.last_user_ts {
            let (ts, email) = last_user_ts.split_at(10);
            (email, ts.parse().unwrap())
        } else {
            ("", 0)
        }
    }

    #[inline]
    fn set_last_user_ts(&mut self, email: &str, created_at: i64) {
        self.last_user_ts = Some(format!("{created_at}{email}"));
    }
}

impl From<EmailJobStatus> for rauthy_api_types::email_jobs::EmailJobStatus {
    fn from(s: EmailJobStatus) -> Self {
        match s {
            EmailJobStatus::Open => Self::Open,
            EmailJobStatus::Finished => Self::Finished,
            EmailJobStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<EmailContentType> for rauthy_api_types::email_jobs::EmailContentType {
    fn from(value: EmailContentType) -> Self {
        match value {
            EmailContentType::Text => Self::Text,
            EmailContentType::Markdown => Self::Markdown,
            EmailContentType::Html => Self::HTML,
        }
    }
}

impl From<rauthy_api_types::email_jobs::EmailContentType> for EmailContentType {
    fn from(value: rauthy_api_types::email_jobs::EmailContentType) -> Self {
        match value {
            rauthy_api_types::email_jobs::EmailContentType::Text => Self::Text,
            rauthy_api_types::email_jobs::EmailContentType::Markdown => Self::Markdown,
            rauthy_api_types::email_jobs::EmailContentType::HTML => Self::Html,
        }
    }
}

impl From<EmailJob> for EmailJobResponse {
    fn from(v: EmailJob) -> Self {
        let mut targets = None;
        let (filter_type, filter_value) = match v.filter {
            EmailJobFilter::Targets(t) => {
                targets = Some(t);
                (EmailJobFilterType::None, None)
            }
            EmailJobFilter::None => (EmailJobFilterType::None, None),
            EmailJobFilter::InGroup(s) => (EmailJobFilterType::InGroup, Some(s)),
            EmailJobFilter::NotInGroup(s) => (EmailJobFilterType::NotInGroup, Some(s)),
            EmailJobFilter::HasRole(s) => (EmailJobFilterType::HasRole, Some(s)),
            EmailJobFilter::HasNotRole(s) => (EmailJobFilterType::HasNotRole, Some(s)),
        };
        let content_type = match v.content_type {
            EmailContentType::Text => rauthy_api_types::email_jobs::EmailContentType::Text,
            EmailContentType::Markdown => rauthy_api_types::email_jobs::EmailContentType::Markdown,
            EmailContentType::Html => rauthy_api_types::email_jobs::EmailContentType::HTML,
        };

        Self {
            id: v.id,
            channel: v.channel,
            scheduled: v.scheduled,
            status: v.status.into(),
            updated: v.updated,
            last_user_ts: v.last_user_ts,
            filter_type,
            filter_value,
            targets,
            content_type,
            subject: v.subject,
            body: v.body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_filter_roundtrip() {
        let t = EmailTargets {
            roles: vec!["admin".into(), "editor".into()],
            groups: vec!["staff".into()],
            users: vec!["abc123".into()],
            not_roles: vec![],
            not_groups: vec!["banned".into()],
        };
        let s = EmailJobFilter::Targets(t.clone()).to_string();
        assert!(s.starts_with("targets_"));
        assert_eq!(
            s.parse::<EmailJobFilter>().unwrap(),
            EmailJobFilter::Targets(t)
        );
    }

    #[test]
    fn legacy_filters_still_parse() {
        assert_eq!(
            "group_staff".parse::<EmailJobFilter>().unwrap(),
            EmailJobFilter::InGroup("staff".into())
        );
        assert_eq!(
            "none".parse::<EmailJobFilter>().unwrap(),
            EmailJobFilter::None
        );
    }
}
