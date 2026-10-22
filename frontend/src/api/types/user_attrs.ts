import type { JsonValue } from '$utils/jsonValue';

export type UserAttrConfigTyp =
    | 'string'
    | 'text'
    | 'email'
    | 'phone'
    | 'number'
    | 'integer'
    | 'boolean'
    | 'date'
    | 'url'
    | 'enum'
    | 'country'
    | 'timezone';

export interface UserAttrConfigEntity {
    name: string;
    desc?: string;
}

export interface UserAttrConfigRequest {
    /// Validation: PATTERN_ATTR
    name: string;
    /// Validation: PATTERN_ATTR_DESC
    desc?: string;
    default_value?: JsonValue;
    typ?: UserAttrConfigTyp;
    user_editable?: boolean;
    /// Validation: valid regex, max 256 characters
    regex?: string;
    /// Validation: max 128 characters
    error_msg?: string;
    /// For `Enum`. Validation: max 64 entries
    options?: string[];
}

export interface UserAttrConfigValueResponse {
    name: string;
    desc?: string;
    default_value?: JsonValue;
    typ?: UserAttrConfigTyp;
    user_editable?: boolean;
    regex?: string;
    error_msg?: string;
    options?: string[];
}

export interface UserAttrConfigResponse {
    values: UserAttrConfigValueResponse[];
}

export interface UserAttrValueResponse {
    key: string;
    value: JsonValue;
}

export interface UserAttrValuesResponse {
    values: UserAttrValueResponse[];
}

export interface UserEditableAttrResponse {
    name: string;
    desc?: string;
    default_value?: JsonValue;
    typ?: UserAttrConfigTyp;
    regex?: string;
    error_msg?: string;
    options?: string[];
    value?: string;
}

export interface UserEditableAttrsResponse {
    values: UserEditableAttrResponse[];
}
