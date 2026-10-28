export type EventLevel = 'info' | 'notice' | 'warning' | 'critical';
export type EventType =
    | 'InvalidLogins'
    | 'IpBlacklisted'
    | 'IpBlacklistRemoved'
    | 'JwksRotated'
    | 'NewUserRegistered'
    | 'NewRauthyAdmin'
    | 'NewRauthyVersion'
    | 'PossibleBruteForce'
    | 'RauthyStarted'
    | 'RauthyHealthy'
    | 'RauthyUnhealthy'
    | 'SecretsMigrated'
    | 'UserEmailChange'
    | 'UserPasswordReset'
    | 'Test'
    | 'BackchannelLogoutFailed'
    | 'ScimTaskFailed'
    | 'ForcedLogout'
    | 'UserLoginRevoke'
    | 'LoginNewLocation'
    | 'SuspiciousApiScan'
    | 'TokenIssued';

export interface EventsRequest {
    /// Unix timestamp in seconds
    from: number;
    /// Unix timestamp in seconds
    until?: number;
    level: EventLevel;
    typ?: EventType;
}

export interface EventResponse {
    id: string;
    timestamp: number;
    level: EventLevel;
    typ: EventType;
    ip?: string;
    data?: number;
    text?: string;
}

/** An entry of the audit log: who did what to what. `at` is in milliseconds. */
export interface AuditEntry {
    id: string;
    at: number;
    /** a user id, `system` or `api_key:<name>` */
    actor: string;
    /** the email of the actor, if known: show the id without it */
    actor_label?: string | null;
    /** `create`, `update`, `delete`, `action`, or a name like `inactivity_reminder_sent` */
    action: string;
    target_type: string;
    target_id?: string | null;
    target_label?: string | null;
    details?: string | null;
}

export interface AuditSummary {
    created_by?: string | null;
    created_by_label?: string | null;
    created_at?: number | null;
    modified_by?: string | null;
    modified_by_label?: string | null;
    modified_at?: number | null;
}

/** A deleted user, kept so the history can still name them */
export interface AuditUserResponse {
    id: string;
    email?: string | null;
    given_name?: string | null;
    family_name?: string | null;
    preferred_username?: string | null;
    phone?: string | null;
    /** seconds */
    deleted_at: number;
    anonymized: boolean;
}
