-- The inactivity reminder looks for users by their last activity (the last login, or the day the
-- account was created). The expression has to be the same as in the query to be used.
CREATE INDEX users_last_activity_idx
    ON users ((COALESCE(last_login, created_at)));

-- Users with reminders left, by when the last one went out. `sent_count` comes first, so users who
-- used up their reminders are not even looked at.
CREATE INDEX inactivity_reminders_due_idx
    ON inactivity_reminders (sent_count, last_sent);
