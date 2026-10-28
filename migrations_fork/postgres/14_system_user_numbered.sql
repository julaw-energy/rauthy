-- System users have numbers as ids and names (101, 102, ...), which no other user can have: a
-- username must not start with a number, and the email domain `system.invalid` is reserved. The
-- first one was created as `system` by the migration before, it becomes 101.
UPDATE users
SET id = '101', email = '101@system.invalid'
WHERE id = 'system';

UPDATE audit_log SET actor = '101' WHERE actor = 'system';
UPDATE audit_log SET target_id = '101' WHERE target_id = 'system';

-- the list of deleted users, newest first
CREATE INDEX audit_users_deleted_idx ON audit_users (deleted_at);
