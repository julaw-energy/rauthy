use rauthy_data::database::DB;
use rauthy_data::email::inactivity::send_inactivity_reminder;
use rauthy_data::entity::audit_log::{AuditLog, SYSTEM_USER_ID};
use rauthy_data::entity::inactivity_reminders::InactivityReminders;
use rauthy_data::entity::users::User;
use rauthy_data::settings;
use rauthy_error::ErrorResponse;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

/// Users reminded per run. The scheduler runs every hour, so a big user base is worked through
/// over a few days instead of getting one burst of mails.
const BATCH: i64 = 100;

/// Every this many runs (hourly, so once a day) every inactive user is looked at, not only the
/// ones who just became inactive. See `InactivityReminders::find_due`.
const FULL_EVERY: u64 = 24;

static RUNS: AtomicU64 = AtomicU64::new(0);
/// The last run used its whole batch, so more users are probably waiting: look at everybody again
static BACKLOG: AtomicBool = AtomicBool::new(false);

/// Reminds users who have not logged in for a long time. Does nothing until an admin turns it on
/// under Config -> New users.
pub async fn inactivity_reminders() {
    time::sleep(Duration::from_secs(60)).await;
    let mut interval = time::interval(Duration::from_secs(3600));

    loop {
        interval.tick().await;

        if !DB::hql().is_leader_cache().await {
            debug!("Running HA mode without being the leader - skipping inactivity_reminders");
            continue;
        }

        if !settings::reminders().enabled {
            continue;
        }

        debug!("Running inactivity_reminders scheduler");
        if let Err(err) = execute().await {
            error!("Error running inactivity_reminders scheduler: {:?}", err);
        }
    }
}

async fn execute() -> Result<(), ErrorResponse> {
    let s = settings::reminders();
    let now = chrono::Utc::now().timestamp();
    let mut sent = 0;

    // the first run after a start is a full one, so is every 24th and any run after a full batch
    let full = BACKLOG.load(Ordering::Relaxed)
        || RUNS
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(FULL_EVERY);
    let due_users = InactivityReminders::find_due(&s, BATCH, full).await?;
    BACKLOG.store(due_users.len() as i64 >= BATCH, Ordering::Relaxed);

    for due in due_users {
        let user = match User::find(due.user_id.clone()).await {
            Ok(u) => u,
            Err(err) => {
                warn!(
                    user_id = due.user_id,
                    ?err,
                    "Cannot load a user due for a reminder"
                );
                continue;
            }
        };
        if user.email.is_empty() {
            continue;
        }

        // Only a reminder that was handed to the mailer is remembered. If that failed, the user
        // is due again in the next run.
        if let Err(err) = send_inactivity_reminder(&user, due.days_inactive(now), &s).await {
            error!(
                user_id = user.id,
                ?err,
                "Could not queue the inactivity reminder"
            );
            continue;
        }
        InactivityReminders::record_sent(&user.id, due.next_count()).await?;
        AuditLog::record_or_log(
            SYSTEM_USER_ID,
            "inactivity_reminder_sent",
            "user",
            Some(&user.id),
            Some(format!(
                "reminder {} of {}, {} days since the last activity",
                due.next_count(),
                s.max_reminders,
                due.days_inactive(now)
            )),
        )
        .await;
        sent += 1;

        // be gentle with the mail server
        time::sleep(Duration::from_millis(300)).await;
    }

    if sent > 0 {
        info!("Sent {sent} inactivity reminders");
    }
    Ok(())
}
