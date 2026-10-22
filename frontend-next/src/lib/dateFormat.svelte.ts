// Display format for date inputs. The underlying value stays ISO `yyyy-mm-dd`;
// only what the user sees changes. Stored per browser.
export type DateFormat = 'dd mmm yyyy' | 'dd/mm/yyyy' | 'yyyy-mm-dd' | 'mm/dd/yyyy';

export const DATE_FORMATS: DateFormat[] = ['dd mmm yyyy', 'dd/mm/yyyy', 'yyyy-mm-dd', 'mm/dd/yyyy'];

const KEY = 'rauthy-date-format';
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

function load(): DateFormat {
    try {
        const v = localStorage.getItem(KEY) as DateFormat | null;
        if (v && DATE_FORMATS.includes(v)) return v;
    } catch {
        // storage unavailable
    }
    return 'dd mmm yyyy';
}

class DateFormatState {
    format = $state<DateFormat>('dd mmm yyyy');
    constructor() {
        if (typeof localStorage !== 'undefined') this.format = load();
    }
    set(f: DateFormat) {
        this.format = f;
        try {
            localStorage.setItem(KEY, f);
        } catch {
            // ignore
        }
    }
}

export const dateFormat = new DateFormatState();

/** `2025-10-22` -> display string according to the chosen format */
export function formatIsoDate(iso?: string | null, f: DateFormat = dateFormat.format): string {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso || '');
    if (!m) return '';
    const [, y, mo, d] = m;
    switch (f) {
        case 'dd/mm/yyyy':
            return `${d}/${mo}/${y}`;
        case 'yyyy-mm-dd':
            return `${y}-${mo}-${d}`;
        case 'mm/dd/yyyy':
            return `${mo}/${d}/${y}`;
        default:
            return `${d} ${MONTHS[Number(mo) - 1]} ${y}`;
    }
}
