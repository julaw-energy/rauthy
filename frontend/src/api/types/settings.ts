export type MessageChannel = 'email' | 'sms' | 'whatsapp';

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
