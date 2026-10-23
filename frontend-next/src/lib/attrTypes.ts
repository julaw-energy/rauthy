import { parsePhoneNumberFromString } from 'libphonenumber-js/min';
import { getTimezone } from 'countries-and-timezones';
import { parseJsonValue, type JsonValue } from '$utils/jsonValue';
import { countryList } from '$lib5/countries';

/** Mirrors `UserAttrConfigTyp` in the backend. An attribute without a type accepts any JSON value. */
export type AttrTyp =
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

export const ATTR_TYPES: { value: AttrTyp; label: string }[] = [
    { value: 'string', label: 'Text (single line)' },
    { value: 'text', label: 'Text (multi line)' },
    { value: 'email', label: 'Email address' },
    { value: 'phone', label: 'Phone number' },
    { value: 'number', label: 'Number' },
    { value: 'integer', label: 'Whole number' },
    { value: 'boolean', label: 'Yes / No' },
    { value: 'date', label: 'Date' },
    { value: 'url', label: 'Web address (URL)' },
    { value: 'enum', label: 'Choice from a list' },
    { value: 'country', label: 'Country' },
    { value: 'timezone', label: 'Timezone' },
];

export interface AttrDef {
    name: string;
    desc?: string;
    typ?: AttrTyp;
    regex?: string;
    error_msg?: string;
    options?: string[];
    default_value?: JsonValue;
}

const DEFAULT_MSG: Record<AttrTyp, string> = {
    string: 'Invalid value',
    text: 'Invalid value',
    email: 'Enter a valid email address',
    phone: 'Enter a phone number with country code, like +256700000000',
    number: 'Enter a number',
    integer: 'Enter a whole number',
    boolean: 'Choose yes or no',
    date: 'Enter a valid date',
    url: 'Enter a web address starting with http:// or https://',
    enum: 'Choose one of the listed values',
    country: 'Choose a country',
    timezone: 'Choose a timezone',
};

const RE_EMAIL = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;
const RE_URL = /^https?:\/\/[^\s/?#]+([/?#]\S*)?$/;

function validDate(s: string) {
    let m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
    if (!m) return false;
    let d = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3]));
    return d.getUTCFullYear() === +m[1] && d.getUTCMonth() === +m[2] - 1 && d.getUTCDate() === +m[3];
}

/** The message to show when `text` is not valid for `def`, `null` if it is (empty means "no value"). */
export function validateAttr(def: AttrDef, text: string | number | null | undefined): string | null {
    let v = String(text ?? '').trim();
    if (!def.typ || !v) return null;

    let ok = true;
    switch (def.typ) {
        case 'email':
            ok = RE_EMAIL.test(v);
            break;
        case 'phone':
            ok = parsePhoneNumberFromString(v)?.isPossible() === true && /^\+[0-9]{8,15}$/.test(v);
            break;
        case 'number':
            ok = Number.isFinite(Number(v));
            break;
        case 'integer':
            ok = /^-?\d+$/.test(v);
            break;
        case 'boolean':
            ok = v === 'true' || v === 'false';
            break;
        case 'date':
            ok = validDate(v);
            break;
        case 'url':
            ok = RE_URL.test(v);
            break;
        case 'enum':
            ok = !!def.options?.includes(v);
            break;
        case 'country':
            ok = countryList().some(c => c.code === v);
            break;
        case 'timezone':
            ok = !!getTimezone(v);
            break;
    }

    if (ok && def.regex) {
        try {
            ok = new RegExp(def.regex).test(v);
        } catch {
            // an invalid pattern is rejected by the backend when it is saved
        }
    }

    return ok ? null : def.error_msg || DEFAULT_MSG[def.typ];
}

/** Backend value -> text for the form fields */
export function attrToText(typ: AttrTyp | undefined, v: JsonValue): string {
    if (v === undefined || v === null) return '';
    if (typ) return typeof v === 'object' ? JSON.stringify(v) : String(v);
    if (typeof v === 'object') return JSON.stringify(v, undefined, 1);
    return String(v);
}

/** Form text -> JSON value for the backend. Typed attributes never go through guessing. */
export function attrFromText(typ: AttrTyp | undefined, text: string | undefined): JsonValue {
    let v = text?.trim();
    if (!v) return undefined;
    switch (typ) {
        case undefined:
            return parseJsonValue(v);
        case 'boolean':
            return v === 'true';
        case 'number':
        case 'integer':
            return Number(v);
        default:
            return v;
    }
}
