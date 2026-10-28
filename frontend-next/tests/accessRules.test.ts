// Run with `pnpm test`. Node runs the TypeScript directly, no extra dependency.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
    changeField,
    changeOp,
    defaultOp,
    emptyRow,
    opsFor,
    problem,
    toRow,
    toRule,
    valueMode,
    type Row,
} from '../src/lib/admin/clients/accessRules.ts';

const kinds = {
    age: 'number',
    country: 'country',
    email_verified: 'bool',
    gender: 'choice',
    email: 'text',
    roles: 'list',
    birthdate: 'date',
} as const;
const kindOf = (f: string) => kinds[f as keyof typeof kinds] ?? 'text';

const row = (over: Partial<Row>): Row => ({ field: 'age', op: 'gte', text: '', text2: '', list: [], message: '', ...over });

test('numbers show math signs and a range', () => {
    const ops = opsFor('number');
    assert.deepEqual(
        ops.map(o => o.label),
        ['=', '≠', '>', '≥', '<', '≤', 'between', 'has a value', 'is empty'],
    );
    assert.ok(ops.some(o => o.value === 'between'));
});

test('text shows words, lists their own words', () => {
    assert.deepEqual(opsFor('text').map(o => o.label).slice(0, 4), ['is', 'is not', 'is one of', 'is none of']);
    assert.equal(opsFor('list').find(o => o.value === 'contains')?.label, 'includes');
});

test('a true / false field only offers "is"', () => {
    assert.deepEqual(opsFor('bool'), [{ value: 'eq', label: 'is' }]);
    assert.equal(valueMode('bool', 'eq'), 'bool');
});

test('the input follows the kind and the condition', () => {
    assert.equal(valueMode('number', 'gte'), 'number');
    assert.equal(valueMode('number', 'between'), 'range');
    assert.equal(valueMode('country', 'in'), 'many');
    assert.equal(valueMode('country', 'eq'), 'one');
    assert.equal(valueMode('text', 'is_set'), 'none');
    assert.equal(valueMode('date', 'eq'), 'date');
    assert.equal(valueMode('list', 'contains'), 'one');
});

test('every kind starts with a condition it offers', () => {
    for (const k of ['number', 'bool', 'date', 'country', 'choice', 'list', 'text'] as const) {
        assert.ok(opsFor(k).some(o => o.value === defaultOp(k)), k);
    }
});

test('rows become rules with the right types', () => {
    assert.deepEqual(toRule(row({ op: 'gte', text: '18' }), 'number'), { field: 'age', op: 'gte', value: 18 });
    assert.deepEqual(toRule(row({ op: 'between', text: '18', text2: '65' }), 'number'), {
        field: 'age',
        op: 'between',
        value: [18, 65],
    });
    assert.deepEqual(toRule(row({ field: 'email_verified', op: 'eq', text: 'true' }), 'bool'), {
        field: 'email_verified',
        op: 'eq',
        value: true,
    });
    assert.deepEqual(toRule(row({ field: 'email_verified', op: 'eq', text: 'false' }), 'bool'), {
        field: 'email_verified',
        op: 'eq',
        value: false,
    });
    assert.deepEqual(toRule(row({ field: 'country', op: 'in', list: ['UG', ' KE ', ''] }), 'country'), {
        field: 'country',
        op: 'in',
        value: ['UG', 'KE'],
    });
    assert.deepEqual(toRule(row({ field: 'email', op: 'ends_with', text: ' @x.com ' }), 'text'), {
        field: 'email',
        op: 'ends_with',
        value: '@x.com',
    });
    assert.deepEqual(toRule(row({ field: 'email', op: 'is_set' }), 'text'), { field: 'email', op: 'is_set' });
});

test('rules become rows and back without change', () => {
    const rules = [
        { field: 'age', op: 'between', value: [18, 65] },
        { field: 'age', op: 'gte', value: 18 },
        { field: 'email_verified', op: 'eq', value: true },
        { field: 'country', op: 'in', value: ['UG', 'KE', 'TZ'] },
        { field: 'email', op: 'ends_with', value: '@example.com' },
        { field: 'attr:member', op: 'is_set' },
    ];
    for (const rule of rules) {
        assert.deepEqual(toRule(toRow(rule), kindOf(rule.field.startsWith('attr:') ? 'x' : rule.field)), rule);
    }
});

test('picking another field starts the value over and fixes the condition', () => {
    // a range on age, then the field becomes the country: "between" does not exist there
    const r = changeField(row({ op: 'between', text: '18', text2: '65' }), 'country', 'country');
    assert.equal(r.field, 'country');
    assert.equal(r.op, 'in');
    assert.equal(r.text, '');
    assert.equal(r.text2, '');
    // a condition both kinds have is kept
    assert.equal(changeField(row({ op: 'eq', text: '18' }), 'email', 'text').op, 'eq');
    // a true / false field starts with "true", so a saved rule never has no value
    assert.equal(emptyRow('email_verified', 'bool').text, 'true');
});

test('changing the condition keeps the value only while the same input fits', () => {
    const kept = changeOp(row({ op: 'gte', text: '18' }), 'gt', 'number');
    assert.equal(kept.text, '18');
    const cleared = changeOp(row({ op: 'gte', text: '18' }), 'between', 'number');
    assert.equal(cleared.text, '');
    assert.equal(cleared.op, 'between');
});

test('problems a person can fix are caught before saving', () => {
    assert.equal(problem([row({ op: 'gte', text: '18' })], kindOf), '');
    assert.match(problem([row({ op: 'gte', text: '' })], kindOf), /needs a number/);
    assert.match(problem([row({ op: 'gte', text: 'old' })], kindOf), /needs a number/);
    assert.match(problem([row({ op: 'between', text: '18', text2: '' })], kindOf), /two numbers/);
    assert.match(problem([row({ op: 'between', text: '65', text2: '18' })], kindOf), /starts above/);
    assert.equal(problem([row({ op: 'between', text: '18', text2: '18' })], kindOf), '', 'a range of one value');
    assert.match(problem([row({ field: 'country', op: 'in', list: [] })], kindOf), /at least one value/);
    assert.match(problem([row({ field: 'email', op: 'eq', text: '  ' })], kindOf), /needs a value/);
    assert.equal(problem([row({ field: 'email', op: 'is_set' })], kindOf), '', 'no value needed');
    assert.equal(problem([row({ field: 'email_verified', op: 'eq', text: 'true' })], kindOf), '');
});

test('the message of a rule is kept, trimmed, and left out when empty', () => {
    const r = row({ field: 'country', op: 'in', list: ['UG'], message: '  Only for Uganda ' });
    assert.deepEqual(toRule(r, 'country'), {
        field: 'country',
        op: 'in',
        value: ['UG'],
        message: 'Only for Uganda'
    });
    assert.equal('message' in toRule(row({ op: 'gte', text: '18', message: '   ' }), 'number'), false);
    assert.equal(toRow({ field: 'age', op: 'gte', value: 18, message: 'Adults only' }).message, 'Adults only');
    assert.equal(toRow({ field: 'age', op: 'gte', value: 18 }).message, '');
    // picking another condition or field does not lose what was written
    assert.equal(changeOp(r, 'not_in', 'country').message, '  Only for Uganda ');
    assert.equal(changeField(r, 'age', 'number').message, '  Only for Uganda ');
    assert.match(problem([row({ op: 'is_set', message: 'x'.repeat(201) })], kindOf), /longer than 200/);
});
