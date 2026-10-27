<script lang="ts">
    import { onMount } from 'svelte';
    import { Plus, Trash2 } from '@lucide/svelte';
    import FormSection from '$lib5/FormSection.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import { fetchGet, fetchPut } from '$api/fetch';
    import type { ClientAccessRule, ClientAccessRulesResponse } from '$api/types/clients';

    // Who may log in to this client, by what their user values and attributes are. All rules must
    // hold. A user without a value for a field never meets a rule on it (except "is empty").
    // The parent calls `save()` together with its own save, `loaded()` says it is safe to.
    let { clientId }: { clientId: string } = $props();

    type Row = { field: string; op: string; text: string };

    const OP_LABELS: Record<string, string> = {
        eq: 'is',
        ne: 'is not',
        gt: 'is greater than',
        gte: 'is at least',
        lt: 'is less than',
        lte: 'is at most',
        in: 'is one of',
        not_in: 'is none of',
        contains: 'contains',
        starts_with: 'starts with',
        ends_with: 'ends with',
        is_set: 'has a value',
        not_set: 'is empty',
    };
    const FIELD_LABELS: Record<string, string> = {
        age: 'age (full years, from the birthdate)',
        email_verified: 'email_verified (true / false)',
        roles: 'roles (a list)',
        groups: 'groups (a list)',
    };

    let rows: Row[] = $state([]);
    let fields: string[] = $state([]);
    let ops: string[] = $state([]);
    let isLoaded = $state(false);
    let err = $state('');

    export function loaded() {
        return isLoaded;
    }

    onMount(load);

    async function load() {
        let res = await fetchGet<ClientAccessRulesResponse>(
            `/auth/v1/clients/${clientId}/access_rules`,
        );
        if (res.body) {
            fields = res.body.fields;
            ops = res.body.ops;
            rows = res.body.rules.map(toRow);
            isLoaded = true;
        } else {
            err = res.error?.message || 'The access rules could not be loaded';
        }
    }

    const takesNothing = (op: string) => op === 'is_set' || op === 'not_set';
    const takesList = (op: string) => op === 'in' || op === 'not_in';
    const isNumeric = (op: string) => ['gt', 'gte', 'lt', 'lte'].includes(op);

    function toRow(r: ClientAccessRule): Row {
        let text = '';
        if (Array.isArray(r.value)) {
            text = r.value.join(', ');
        } else if (r.value !== undefined && r.value !== null) {
            text = String(r.value);
        }
        return { field: r.field, op: r.op, text };
    }

    function toRule(r: Row): ClientAccessRule {
        if (takesNothing(r.op)) {
            return { field: r.field, op: r.op };
        }
        if (takesList(r.op)) {
            return {
                field: r.field,
                op: r.op,
                value: r.text
                    .split(',')
                    .map(s => s.trim())
                    .filter(s => s.length > 0),
            };
        }
        if (isNumeric(r.op)) {
            return { field: r.field, op: r.op, value: Number(r.text.trim()) };
        }
        return { field: r.field, op: r.op, value: r.text.trim() };
    }

    /** the first problem a person can fix before the server is asked, or empty */
    function problem(): string {
        for (let r of rows) {
            if (!r.field) return 'Every rule needs a field';
            if (takesNothing(r.op)) continue;
            if (r.text.trim() === '') return `The rule on ${r.field} needs a value`;
            if (isNumeric(r.op) && !Number.isFinite(Number(r.text.trim()))) {
                return `The rule on ${r.field} compares numbers, "${r.text}" is not one`;
            }
        }
        return '';
    }

    /** Saves the rules, returns an error message or an empty string. Does nothing before they were loaded. */
    export async function save(): Promise<string> {
        if (!isLoaded) return '';
        let p = problem();
        if (p) return p;

        let res = await fetchPut<ClientAccessRulesResponse>(
            `/auth/v1/clients/${clientId}/access_rules`,
            { rules: rows.map(toRule) },
        );
        if (res.error) return res.error.message;
        if (res.body) rows = res.body.rules.map(toRow);
        return '';
    }

    function add() {
        rows = [...rows, { field: 'country', op: 'in', text: '' }];
    }

    function remove(i: number) {
        rows = rows.filter((_, idx) => idx !== i);
    }

    function hint(r: Row): string {
        if (r.field === 'country') return 'for example UG, KE, TZ';
        if (r.field === 'age') return 'for example 18';
        if (takesList(r.op)) return 'separate values with a comma';
        return '';
    }
</script>

<FormSection
    title="Access rules"
    desc="Only users who meet <b>all</b> of these rules can log in to this client, for example <code>country is one of UG, KE, TZ</code> or <code>age is at least 18</code>. The rules are checked at every login and token refresh. A user who has no value for a field does not meet a rule on it."
>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    {#each rows as r, i (i)}
        <div class="grid items-end gap-2 sm:grid-cols-[1.2fr_1fr_1.4fr_auto]">
            <InputSelect
                bind:value={r.field}
                label={i === 0 ? 'Field' : ''}
                items={fields.map(f => ({ value: f, label: FIELD_LABELS[f] ?? f }))}
            />
            <InputSelect
                bind:value={r.op}
                label={i === 0 ? 'Condition' : ''}
                items={ops.map(o => ({ value: o, label: OP_LABELS[o] ?? o }))}
            />
            {#if takesNothing(r.op)}
                <div></div>
            {:else}
                <Input
                    bind:value={r.text}
                    label={i === 0 ? 'Value' : ''}
                    autocomplete="off"
                    placeholder={hint(r)}
                />
            {/if}
            <Button invisible ariaLabel="Remove this rule" onclick={() => remove(i)}>
                <Trash2 class="size-4" />
            </Button>
        </div>
    {/each}

    {#if isLoaded}
        <div>
            <Button level={2} onclick={add}>
                <span class="inline-flex items-center gap-1.5"><Plus class="size-4" /> Add a rule</span>
            </Button>
        </div>
    {/if}
</FormSection>
