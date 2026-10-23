export type MessageChannel = 'email' | 'sms' | 'whatsapp';

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

export interface ChannelSettingsResponse {
    channel: MessageChannel;
    /// '' (disabled) | 'smtp' (email only) | 'notifyd' | 'webhook'
    driver: string;
    url?: string;
    has_api_key: boolean;
    channel_name: string;
    from_db: boolean;
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

export interface ChannelSettingsRequest {
    driver: string;
    url?: string;
    /// undefined keeps the stored key, an empty string removes it
    api_key?: string;
    channel_name?: string;
}

export interface SettingsTestResponse {
    message: string;
}
