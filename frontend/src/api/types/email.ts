export type EmailContentType = 'text' | 'markdown' | 'html';
export type EmailJobFilterType = 'none' | 'in_group' | 'not_in_group' | 'has_role' | 'has_not_role';
export type EmailJobStatus = 'Open' | 'Finished' | 'Canceled';

/// Multi-select recipients: a user matches if it is in any of roles / groups / users (or if all
/// three are empty), and in none of not_roles / not_groups
export interface EmailTargets {
    roles: string[];
    groups: string[];
    /// user IDs
    users: string[];
    not_roles: string[];
    not_groups: string[];
}

export type MessageChannel = string;

export interface ChannelInfo {
    channel: MessageChannel;
    /// 'smtp' | 'webhook' | 'http'
    driver: string;
    /// true if the channel can send HTML content
    html: boolean;
    label: string;
    /// the user value the channel delivers to: 'email' | 'phone' | 'attr:<name>'
    recipient: string;
}

export interface EmailJobRequest {
    /// defaults to 'email'. SMS and WhatsApp only support content_type 'text'
    channel?: MessageChannel;
    /// Validation: Unix TS in the future
    scheduled?: number;
    filter_type: EmailJobFilterType;
    /// Validation: `^[a-zA-Z0-9-_/,:*\s]{2,64}$`
    filter_value?: string;
    /// takes precedence over filter_type if given and not empty
    targets?: EmailTargets;
    content_type: EmailContentType;
    /// Validation: max length 1024
    subject: string;
    /// Validation: max length 32768
    body: string;
}

export interface EmailJobResponse {
    id: number;
    channel: MessageChannel;
    scheduled?: number;
    status: EmailJobStatus;
    updated: number;
    last_user_ts?: number;
    filter_type: EmailJobFilterType;
    filter_value?: string;
    targets?: EmailTargets;
    content_type: EmailContentType;
    subject: string;
    body: string;
}
