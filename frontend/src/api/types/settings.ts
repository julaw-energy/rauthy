/// 'email' and 'sms' always exist, an admin can add more
export type MessageChannel = string;

export type UserValueRequirement = 'required' | 'optional' | 'hidden';

export interface UserValuesSettings {
    given_name: UserValueRequirement;
    family_name: UserValueRequirement;
    birthdate: UserValueRequirement;
    gender: UserValueRequirement;
    street: UserValueRequirement;
    zip: UserValueRequirement;
    city: UserValueRequirement;
    country: UserValueRequirement;
    phone: UserValueRequirement;
    tz: UserValueRequirement;
    /// ask users to confirm / complete their profile again at every client login
    revalidate_during_login: boolean;
    /// show the family name before the given name
    name_order_reverse: boolean;
}

export interface UserValuesSettingsResponse extends UserValuesSettings {
    /// true if the values come from the database, false if from the config file
    from_db: boolean;
}

/// All durations are in seconds
export interface LifetimesSettings {
    refresh_token_grace_time: number;
    refresh_token_lifetime: number;
    session_lifetime: number;
    session_timeout: number;
    session_renew_mfa: boolean;
    magic_link_pwd_reset: number;
    magic_link_pwd_first: number;
}

export interface LifetimesSettingsResponse extends LifetimesSettings {
    from_db: boolean;
}

export interface GeneralSettingsResponse {
    /// the name users see instead of "Rauthy"
    app_name: string;
    /// true if the value comes from the database, false if from the config file
    from_db: boolean;
}

export interface GeneralSettingsRequest {
    app_name: string;
}

export interface SmtpSettingsResponse {
    url?: string;
    port?: number;
    username?: string;
    has_password: boolean;
    from: string;
    /// 'tls' | 'starttls' | 'danger-insecure'
    tls_mode: string;
    /// true if the values come from the database, false if from the config file
    from_db: boolean;
}

export interface HttpKvResponse {
    name: string;
    /// undefined for a secret, it never leaves the backend
    value?: string;
    secret: boolean;
}

export interface HttpSettingsResponse {
    /// 'POST' | 'PUT' | 'PATCH'
    method: string;
    url: string;
    headers: HttpKvResponse[];
    /// 'json' | 'form'
    body_format: string;
    body: HttpKvResponse[];
}

export interface ChannelSettingsResponse {
    channel: MessageChannel;
    /// '' (disabled) | 'smtp' | 'webhook' | 'http'
    driver: string;
    /// webhook
    url?: string;
    /// webhook: a signing secret is set
    has_api_key: boolean;
    from_db: boolean;
    /// shown instead of the name
    label: string;
    /// the user value this channel delivers to: 'email' | 'phone' | 'attr:<name>'
    recipient: string;
    /// 'email' and 'sms' cannot be deleted
    builtin: boolean;
    /// Unix timestamp after which the channel stops working (custom channels)
    expires?: number;
    expired: boolean;
    /// smtp of a custom channel: its own server
    smtp?: SmtpSettingsResponse;
    http?: HttpSettingsResponse;
}

export interface SettingsResponse {
    general: GeneralSettingsResponse;
    user_values: UserValuesSettingsResponse;
    lifetimes: LifetimesSettingsResponse;
    smtp: SmtpSettingsResponse;
    /// false while no SMTP server is set up
    smtp_configured: boolean;
    channels: ChannelSettingsResponse[];
}

export interface SmtpSettingsRequest {
    url?: string;
    port?: number;
    username?: string;
    /// undefined keeps the stored password, an empty string removes it
    password?: string;
    from: string;
    tls_mode: string;
}

export interface HttpKvRequest {
    name: string;
    /// for a secret, undefined keeps the stored value of the entry with the same name
    value?: string;
    secret: boolean;
}

export interface HttpSettingsRequest {
    method: string;
    url: string;
    headers: HttpKvRequest[];
    /// 'json' | 'form'. A body name like `messages.0.to` builds nested JSON.
    body_format: string;
    body: HttpKvRequest[];
}

export interface ChannelSettingsRequest {
    driver: string;
    url?: string;
    /// webhook signing secret. undefined keeps the stored one, an empty string removes it
    api_key?: string;
    label?: string;
    recipient?: string;
    /// custom channels only, undefined: never expires
    expires?: number;
    smtp?: SmtpSettingsRequest;
    http?: HttpSettingsRequest;
}

export interface SettingsTestResponse {
    message: string;
}

export interface StaticSettingEntry {
    key: string;
    value: string;
    secret: boolean;
}

export interface StaticSettingsSection {
    title: string;
    entries: StaticSettingEntry[];
}

export interface StaticSettingsResponse {
    sections: StaticSettingsSection[];
}
