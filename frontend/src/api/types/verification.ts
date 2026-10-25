export type ContactKind = 'email' | 'phone' | 'other';

export interface ChannelOption {
    name: string;
    label: string;
}

export interface ContactStatus {
    value: string;
    verified: boolean;
}

export interface VerificationStatusResponse {
    email: ContactStatus;
    /// undefined if the user has no phone number
    phone?: ContactStatus;
    /// true if an SMS channel is configured, so a phone number can be verified by code
    phone_verify_available: boolean;
    /// the first one is the default
    email_channels: ChannelOption[];
    phone_channels: ChannelOption[];
}

export interface AttrVerificationResponse {
    kind: ContactKind;
    /// undefined if the user has no value for the attribute yet
    value?: string;
    verified: boolean;
    /// false if the needed channel (SMS for phone numbers) is not configured
    verify_available: boolean;
    /// the channels the user can pick, the first one is the default
    channels: ChannelOption[];
}

export interface AttrVerifyChannelsResponse {
    /// linked, in order of preference
    channels: string[];
    /// all channels that are able to deliver to this attribute
    candidates: ChannelOption[];
}

export interface VerifyConfirmRequest {
    /// 6 digits
    code: string;
}

export interface VerifySetRequest {
    verified: boolean;
}
