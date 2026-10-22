import { getCountries } from 'libphonenumber-js/min';
import { getCountry, getTimezone } from 'countries-and-timezones';

export interface Country {
    code: string;
    name: string;
}

export const flag = (c: string) =>
    String.fromCodePoint(...[...c.toUpperCase()].map(ch => 127397 + ch.charCodeAt(0)));

let cache: Country[] | undefined;

/** ISO 3166-1 alpha-2 countries, names localized by the browser, sorted by name */
export function countryList(): Country[] {
    if (cache) return cache;
    let names: Intl.DisplayNames | undefined;
    try {
        names = new Intl.DisplayNames([navigator.language || 'en'], { type: 'region' });
    } catch {
        // fall back to codes
    }
    cache = getCountries()
        .map(code => ({ code: code as string, name: names?.of(code) ?? code }))
        .sort((a, b) => a.name.localeCompare(b.name));
    return cache;
}

/** Resolve stored free text ("KE", "ke", "Kenya") to an ISO code, if it matches one */
export function resolveCountry(v?: string | null): string | undefined {
    const s = (v ?? '').trim().toLowerCase();
    if (!s) return undefined;
    return countryList().find(c => c.code.toLowerCase() === s || c.name.toLowerCase() === s)?.code;
}

// Countries with several zones where "first in the list" is a poor default: use the most populous / capital zone.
const MAIN_TZ: Record<string, string> = {
    US: 'America/New_York',
    CA: 'America/Toronto',
    BR: 'America/Sao_Paulo',
    MX: 'America/Mexico_City',
    AR: 'America/Argentina/Buenos_Aires',
    CL: 'America/Santiago',
    EC: 'America/Guayaquil',
    RU: 'Europe/Moscow',
    UA: 'Europe/Kyiv',
    ES: 'Europe/Madrid',
    PT: 'Europe/Lisbon',
    AU: 'Australia/Sydney',
    NZ: 'Pacific/Auckland',
    CN: 'Asia/Shanghai',
    ID: 'Asia/Jakarta',
    KZ: 'Asia/Almaty',
    MN: 'Asia/Ulaanbaatar',
    CD: 'Africa/Kinshasa',
    GL: 'America/Nuuk',
    FM: 'Pacific/Pohnpei',
    GB: 'Europe/London',
};

// obsolete spellings / old links that must not be offered as a default
const LEGACY_TZ = new Set(['Pacific/Samoa', 'Pacific/Ponape', 'Africa/Asmera', 'Africa/Timbuktu', 'Europe/Belfast']);

/**
 * Main IANA timezone of a country, if known. Many small countries only appear in the tz database as
 * a link to a shared zone (Uganda -> Africa/Nairobi); for those the country-specific name
 * (Africa/Kampala) is preferred, since that is what the backend and users expect to see.
 */
export function firstTimezone(code?: string | null): string | undefined {
    if (!code) return undefined;
    const cc = code.toUpperCase();
    if (MAIN_TZ[cc]) return MAIN_TZ[cc];

    const main = getCountry(cc)?.timezones?.[0];
    if (!main) return undefined;

    const shared = (getTimezone(main)?.countries?.length ?? 0) > 1;
    if (shared) {
        const own = getCountry(cc, { deprecated: true })?.timezones?.find(z => {
            const tz = getTimezone(z);
            return (
                z.includes('/') &&
                !LEGACY_TZ.has(z) &&
                tz?.aliasOf === main &&
                tz.countries.length === 1 &&
                tz.countries[0] === cc
            );
        });
        if (own) return own;
    }
    return main;
}

/**
 * New timezone to autofill after the country changed, or `undefined` to leave it alone.
 * A timezone the user chose on purpose (neither unset/UTC nor the previous country's default) is kept.
 */
export function tzAfterCountryChange(
    current: string | undefined | null,
    prevCountry: string | undefined | null,
    nextCountry: string,
): string | undefined {
    const next = firstTimezone(nextCountry);
    if (!next) return undefined;
    const untouched = !current || current === 'UTC' || current === 'Etc/UTC' || current === firstTimezone(prevCountry);
    return untouched ? next : undefined;
}

/** IANA timezone reported by the browser (device setting, unaffected by a VPN) */
export function browserTimezone(): string | undefined {
    try {
        return Intl.DateTimeFormat().resolvedOptions().timeZone || undefined;
    } catch {
        return undefined;
    }
}

/**
 * Best-effort country from the browser timezone. Returns a code only when it is unambiguous:
 * the zone belongs to one country, or the browser's language regions pick exactly one of the
 * countries sharing it. Otherwise `undefined`; the user chooses.
 */
export function countryFromBrowser(): string | undefined {
    const tz = browserTimezone();
    const candidates = tz ? getTimezone(tz)?.countries : undefined;
    if (!candidates?.length) return undefined;
    if (candidates.length === 1) return candidates[0];

    const regions = (navigator.languages?.length ? navigator.languages : [navigator.language])
        .map(l => {
            try {
                return new Intl.Locale(l).region;
            } catch {
                return undefined;
            }
        })
        .filter((r): r is string => !!r);
    const hit = candidates.filter(c => regions.includes(c));
    return hit.length === 1 ? hit[0] : undefined;
}
