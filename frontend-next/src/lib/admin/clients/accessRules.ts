import type { AccessRuleFieldKind, ClientAccessRule } from '$api/types/clients';

// What the access rule editor offers for each kind of field, and how a row becomes a rule and back.
// Pure functions, so they can be tested without a browser.

export type Row = {
    field: string;
    op: string;
    /** the value, or the lower end of a range. `true` / `false` for a true / false field. */
    text: string;
    /** the upper end of a range */
    text2: string;
    /** the values of `is one of` and `is none of` */
    list: string[];
};

export type OpItem = { value: string; label: string };

/** words for text, math signs for numbers */
const WORDS: Record<string, string> = {
    eq: 'is',
    ne: 'is not',
    in: 'is one of',
    not_in: 'is none of',
    contains: 'contains',
    starts_with: 'starts with',
    ends_with: 'ends with',
    is_set: 'has a value',
    not_set: 'is empty',
};
const MATH: Record<string, string> = {
    eq: '=',
    ne: '≠',
    gt: '>',
    gte: '≥',
    lt: '<',
    lte: '≤',
    between: 'between',
    is_set: 'has a value',
    not_set: 'is empty',
};
const LIST_WORDS: Record<string, string> = {
    contains: 'includes',
    in: 'includes any of',
    not_in: 'includes none of',
    is_set: 'has some',
    not_set: 'is empty',
};

const OPS: Record<AccessRuleFieldKind, string[]> = {
    number: ['eq', 'ne', 'gt', 'gte', 'lt', 'lte', 'between', 'is_set', 'not_set'],
    bool: ['eq'],
    date: ['eq', 'ne', 'is_set', 'not_set'],
    country: ['in', 'not_in', 'eq', 'ne', 'is_set', 'not_set'],
    choice: ['in', 'not_in', 'eq', 'ne', 'is_set', 'not_set'],
    list: ['contains', 'in', 'not_in', 'is_set', 'not_set'],
    text: ['eq', 'ne', 'in', 'not_in', 'contains', 'starts_with', 'ends_with', 'is_set', 'not_set'],
};

/** the conditions that make sense for a kind of field, in the order they are shown */
export function opsFor(kind: AccessRuleFieldKind): OpItem[] {
    const labels = kind === 'number' ? MATH : kind === 'list' ? LIST_WORDS : WORDS;
    return OPS[kind].map(value => ({ value, label: labels[value] ?? value }));
}

/** what a new rule on this kind of field starts with: the most common condition */
export function defaultOp(kind: AccessRuleFieldKind): string {
    switch (kind) {
        case 'number':
            return 'gte';
        case 'country':
        case 'choice':
            return 'in';
        case 'list':
            return 'contains';
        default:
            return 'eq';
    }
}

export type ValueMode = 'none' | 'bool' | 'number' | 'range' | 'date' | 'many' | 'one';

/** which input a rule needs for its value */
export function valueMode(kind: AccessRuleFieldKind, op: string): ValueMode {
    if (op === 'is_set' || op === 'not_set') return 'none';
    if (kind === 'bool') return 'bool';
    if (op === 'between') return 'range';
    if (kind === 'number') return 'number';
    if (op === 'in' || op === 'not_in') return 'many';
    if (kind === 'date') return 'date';
    return 'one';
}

export function emptyRow(field: string, kind: AccessRuleFieldKind): Row {
    return {
        field,
        op: defaultOp(kind),
        text: kind === 'bool' ? 'true' : '',
        text2: '',
        list: [],
    };
}

/** The row after another field was picked: a condition that does not fit the new kind is replaced, the value starts over. */
export function changeField(row: Row, field: string, kind: AccessRuleFieldKind): Row {
    const keep = opsFor(kind).some(o => o.value === row.op);
    const fresh = emptyRow(field, kind);
    return keep ? { ...fresh, op: row.op } : fresh;
}

/** The row after another condition was picked: the value is kept when the same input still fits. */
export function changeOp(row: Row, op: string, kind: AccessRuleFieldKind): Row {
    if (valueMode(kind, row.op) === valueMode(kind, op)) return { ...row, op };
    return { ...emptyRow(row.field, kind), op };
}

export function toRule(row: Row, kind: AccessRuleFieldKind): ClientAccessRule {
    const { field, op } = row;
    switch (valueMode(kind, op)) {
        case 'none':
            return { field, op };
        case 'bool':
            return { field, op, value: row.text !== 'false' };
        case 'number':
            return { field, op, value: Number(row.text.trim()) };
        case 'range':
            return { field, op, value: [Number(row.text.trim()), Number(row.text2.trim())] };
        case 'many':
            return { field, op, value: row.list.map(s => s.trim()).filter(s => s.length > 0) };
        default:
            return { field, op, value: row.text.trim() };
    }
}

export function toRow(rule: ClientAccessRule): Row {
    const row: Row = { field: rule.field, op: rule.op, text: '', text2: '', list: [] };
    const v = rule.value;
    if (rule.op === 'between' && Array.isArray(v)) {
        row.text = String(v[0] ?? '');
        row.text2 = String(v[1] ?? '');
    } else if (Array.isArray(v)) {
        row.list = v.map(String);
    } else if (v !== undefined && v !== null) {
        row.text = String(v);
    }
    return row;
}

const isNumber = (s: string) => s.trim() !== '' && Number.isFinite(Number(s.trim()));

/** The first thing a person can fix before the server is asked, or an empty text */
export function problem(rows: Row[], kindOf: (field: string) => AccessRuleFieldKind): string {
    for (const r of rows) {
        if (!r.field) return 'Every rule needs a field';
        switch (valueMode(kindOf(r.field), r.op)) {
            case 'number':
                if (!isNumber(r.text)) return `The rule on ${r.field} needs a number`;
                break;
            case 'range':
                if (!isNumber(r.text) || !isNumber(r.text2)) {
                    return `The range on ${r.field} needs two numbers`;
                }
                if (Number(r.text) > Number(r.text2)) {
                    return `The range on ${r.field} starts above where it ends`;
                }
                break;
            case 'many':
                if (r.list.length === 0) return `The rule on ${r.field} needs at least one value`;
                break;
            case 'date':
            case 'one':
                if (r.text.trim() === '') return `The rule on ${r.field} needs a value`;
                break;
        }
    }
    return '';
}
