/** The tab that shows who created something and who changed it, in the popup of the thing. */
export const HISTORY_TAB = 'Status Change history';

/**
 * System users have a short number as id (101, 102, ...), the ids of all other users are long
 * random strings. They are read only: the server refuses every change to them as well.
 * Keep in step with `is_system_user_id` in `src/data/src/entity/audit_log.rs`.
 */
export function isSystemUserId(id: string | undefined | null): boolean {
    return !!id && /^[0-9]{1,8}$/.test(id);
}
