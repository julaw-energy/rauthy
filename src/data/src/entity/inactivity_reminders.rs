//! Who got a reminder for not having logged in for a long time, when, and how many in a row.
//!
//! A user is due when their last activity (the last login, or the day the account was created if
//! they never logged in) is at least `after_days` ago, and
//!
//! - they never got a reminder, or
//! - they were active again after the last reminder (a new round starts), or
//! - the last reminder is at least `after_days` old and fewer than `max_reminders` were sent in
//!   this round.
//!
//! So a user gets at most `max_reminders` reminders, `after_days` apart, until they log in again.

use crate::database::DB;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::settings::ReminderSettings;
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::ErrorResponse;
use serde::Deserialize;

const DAY: i64 = 24 * 3600;

/// Raw `inactivity_reminders` row, used by the DB migration tooling
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct InactivityReminderRow {
    pub user_id: String,
    pub last_sent: i64,
    pub sent_count: i64,
}

/// A user who is due for a reminder
#[derive(Debug, FromRow, FromPgRow)]
pub struct DueUser {
    pub user_id: String,
    /// the last login, or the creation of the account
    pub last_activity: i64,
    pub prev_last_sent: Option<i64>,
    pub prev_count: Option<i64>,
}

impl DueUser {
    /// How many reminders this user will have had, with the one that is about to be sent
    pub fn next_count(&self) -> i64 {
        next_count(self.last_activity, self.prev_last_sent, self.prev_count)
    }

    /// Full days since the last activity
    pub fn days_inactive(&self, now: i64) -> i64 {
        ((now - self.last_activity) / DAY).max(0)
    }
}

/// A reminder after the user was active again starts a new round with 1
fn next_count(last_activity: i64, prev_last_sent: Option<i64>, prev_count: Option<i64>) -> i64 {
    match (prev_last_sent, prev_count) {
        (Some(sent), Some(count)) if sent >= last_activity => count + 1,
        _ => 1,
    }
}

/// Users who are inactive since `$1` .. `$2` and never got a reminder, or were active again after
/// the last one (a new round). Params: lower, cutoff, enabled, now. Needs `users_last_activity_idx`.
///
/// Hiqlite binds the placeholders by the order they appear in: keep them in ascending order.
const NEW_ROUND: &str = r#"
FROM users u
LEFT JOIN inactivity_reminders r ON r.user_id = u.id
WHERE COALESCE(u.last_login, u.created_at) BETWEEN $1 AND $2
  AND u.enabled = $3
  AND (u.user_expires IS NULL OR u.user_expires > $4)
  AND (r.user_id IS NULL OR r.last_sent < COALESCE(u.last_login, u.created_at))"#;

/// Users in a running round whose next reminder is due. Params: max, cutoff, enabled, now. Needs
/// `inactivity_reminders_due_idx`: users with all their reminders sent are skipped by the index.
///
/// Hiqlite binds the placeholders by the order they appear in: keep them in ascending order.
const NEXT_IN_ROUND: &str = r#"
FROM inactivity_reminders r
JOIN users u ON u.id = r.user_id
WHERE r.sent_count < $1
  AND r.last_sent <= $2
  AND r.last_sent >= COALESCE(u.last_login, u.created_at)
  AND u.enabled = $3
  AND (u.user_expires IS NULL OR u.user_expires > $4)"#;

/// How far back the regular run looks for users who just became inactive. Longer than the time
/// between two runs, so a pause (a restart, an outage) loses nobody. The daily full run catches
/// the rest.
const RECENT_WINDOW: i64 = 3 * DAY;

pub struct InactivityReminders;

impl InactivityReminders {
    /// The users that are due, those in a running round first.
    ///
    /// `full` looks at every inactive user. Without it, only users who became inactive during
    /// the last [`RECENT_WINDOW`] are looked at for a first reminder, which is what keeps the
    /// regular run cheap when there are millions of users and most of them are long gone.
    pub async fn find_due(
        s: &ReminderSettings,
        limit: i64,
        full: bool,
    ) -> Result<Vec<DueUser>, ErrorResponse> {
        let now = Utc::now().timestamp();
        let cutoff = now - s.after_days as i64 * DAY;
        let max = s.max_reminders as i64;
        let lower = if full { 0 } else { cutoff - RECENT_WINDOW };

        let cols = r#"
SELECT u.id AS user_id,
       COALESCE(u.last_login, u.created_at) AS last_activity,
       r.last_sent AS prev_last_sent,
       r.sent_count AS prev_count"#;

        let next_sql = format!("{cols}{NEXT_IN_ROUND}\nORDER BY r.last_sent ASC\nLIMIT $5");
        let mut due: Vec<DueUser> = if is_hiqlite() {
            DB::hql()
                .query_map(next_sql, params!(max, cutoff, true, now, limit))
                .await?
        } else {
            DB::pg_query(&next_sql, &[&max, &cutoff, &true, &now, &limit], 0).await?
        };

        let left = limit - due.len() as i64;
        if left > 0 {
            let new_sql = format!("{cols}{NEW_ROUND}\nORDER BY last_activity ASC\nLIMIT $5");
            let new: Vec<DueUser> = if is_hiqlite() {
                DB::hql()
                    .query_map(new_sql, params!(lower, cutoff, true, now, left))
                    .await?
            } else {
                DB::pg_query(&new_sql, &[&lower, &cutoff, &true, &now, &left], 0).await?
            };
            due.extend(new);
        }
        Ok(due)
    }

    /// How many users are due right now, for the admin UI
    pub async fn count_due(s: &ReminderSettings) -> Result<u32, ErrorResponse> {
        #[derive(Debug, FromRow, FromPgRow)]
        struct Count {
            n: i64,
        }

        let now = Utc::now().timestamp();
        let cutoff = now - s.after_days as i64 * DAY;
        let max = s.max_reminders as i64;

        let next_sql = format!("SELECT COUNT(*) AS n {NEXT_IN_ROUND}");
        let new_sql = format!("SELECT COUNT(*) AS n {NEW_ROUND}");
        let (next, new): (Vec<Count>, Vec<Count>) = if is_hiqlite() {
            (
                DB::hql()
                    .query_map(next_sql, params!(max, cutoff, true, now))
                    .await?,
                DB::hql()
                    .query_map(new_sql, params!(0i64, cutoff, true, now))
                    .await?,
            )
        } else {
            (
                DB::pg_query(&next_sql, &[&max, &cutoff, &true, &now], 1).await?,
                DB::pg_query(&new_sql, &[&0i64, &cutoff, &true, &now], 1).await?,
            )
        };
        let n = |v: &[Count]| v.first().map(|c| c.n).unwrap_or_default().max(0);
        Ok((n(&next) + n(&new)).min(u32::MAX as i64) as u32)
    }

    /// Remembers that the reminder was sent
    pub async fn record_sent(user_id: &str, count: i64) -> Result<(), ErrorResponse> {
        let now = Utc::now().timestamp();
        let sql = r#"
INSERT INTO inactivity_reminders (user_id, last_sent, sent_count)
VALUES ($1, $2, $3)
ON CONFLICT (user_id) DO UPDATE SET last_sent = $2, sent_count = $3"#;
        if is_hiqlite() {
            DB::hql().execute(sql, params!(user_id, now, count)).await?;
        } else {
            DB::pg_execute(sql, &[&user_id, &now, &count]).await?;
        }
        Ok(())
    }

    pub async fn find_all() -> Result<Vec<InactivityReminderRow>, ErrorResponse> {
        let sql = "SELECT user_id, last_sent, sent_count FROM inactivity_reminders";
        Ok(if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(sql, &[], 0).await?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn the_first_reminder_is_number_one() {
        assert_eq!(next_count(NOW - 40 * DAY, None, None), 1);
    }

    #[test]
    fn a_reminder_in_the_same_round_counts_up() {
        // last activity 100 days ago, reminders at 70 and 40 days ago
        let a = NOW - 100 * DAY;
        assert_eq!(next_count(a, Some(NOW - 70 * DAY), Some(1)), 2);
        assert_eq!(next_count(a, Some(NOW - 40 * DAY), Some(2)), 3);
    }

    #[test]
    fn activity_after_the_last_reminder_starts_a_new_round() {
        // they were reminded 200 days ago, logged in 50 days ago
        assert_eq!(
            next_count(NOW - 50 * DAY, Some(NOW - 200 * DAY), Some(3)),
            1
        );
    }

    #[test]
    fn days_inactive_are_full_days_and_never_negative() {
        let u = |last_activity| DueUser {
            user_id: "u".into(),
            last_activity,
            prev_last_sent: None,
            prev_count: None,
        };
        assert_eq!(u(NOW - 45 * DAY - 100).days_inactive(NOW), 45);
        assert_eq!(u(NOW + 10).days_inactive(NOW), 0);
    }
}
