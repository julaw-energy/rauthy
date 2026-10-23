export type ContactKind = 'email' | 'phone';

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
}

export interface AttrVerificationResponse {
    kind: ContactKind;
    /// undefined if the user has no value for the attribute yet
    value?: string;
    verified: boolean;
    /// false if the needed channel (SMS for phone numbers) is not configured
    verify_available: boolean;
}

export interface VerifyConfirmRequest {
    /// 6 digits
    code: string;
}

export interface VerifySetRequest {
    verified: boolean;
}
