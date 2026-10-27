<script lang="ts">
    import { onMount } from 'svelte';
    import { Plus, Trash2 } from '@lucide/svelte';
    import FormSection from '$lib5/FormSection.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputCountry from '$lib5/form/InputCountry.svelte';
    import InputDate from '$lib5/form/InputDate.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import MultiPick from '$lib5/form/MultiPick.svelte';
    import { countryList, flag } from '$lib5/countries';
    import { fetchGet, fetchPut } from '$api/fetch';
    import type {
        AccessRuleField,
        AccessRuleFieldKind,
        ClientAccessRulesResponse,
    } from '$api/types/clients';
    import {
        changeField,
        changeOp,
        emptyRow,
        opsFor,
        problem,
        toRow,
        toRule,
        valueMode,
        type Row,
    } from './accessRules';

    // Who may log in to this client, by what their user values and attributes are. All rules must
    // hold. A user without a value for a field never meets a rule on it (except "is empty").
    // The input of every rule follows the kind of its field: a number gets math signs and a range,
    // a true / false field a true / false choice, a country the country picker, and so on.
    // The parent calls `save()` together with its own save.
    let { clientId }: { clientId: string } = $props();

    const FIELD_LABELS: Record<string, string> = {
        age: 'age (full years)',
        email_verified: 'email verified',
        given_name: 'given name',
        family_name: 'family name',
        preferred_username: 'username',
        tz: 'time zone',
        birthdate: 'birthdate',
        roles: 'roles',
        groups: 'groups',
    };

    let rows: Row[] = $state([]);
    let fields: AccessRuleField[] = $state([]);
    let isLoaded = $state(false);
    let err = $state('');

    const countries = countryList().map(c => ({ value: c.code, label: c.name, prefix: flag(c.code) }));

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
            rows = res.body.rules.map(toRow);
            isLoaded = true;
        } else {
            err = res.error?.message || 'The access rules could not be loaded';
        }
    }

    const fieldOf = (name: string) => fields.find(f => f.name === name);
    const kindOf = (name: string): AccessRuleFieldKind => fieldOf(name)?.kind ?? 'text';
    const optionItems = (name: string) =>
        (fieldOf(name)?.options ?? []).map(o => ({ value: o, label: o }));

    function fieldLabel(f: AccessRuleField): string {
        if (f.name.startsWith('attr:')) return `${f.name.slice(5)} (attribute)`;
        return FIELD_LABELS[f.name] ?? f.name.replace(/_/g, ' ');
    }

    /** Saves the rules, returns an error message or an empty string. Does nothing before they were loaded. */
    export async function save(): Promise<string> {
        if (!isLoaded) return '';
        let p = problem(rows, kindOf);
        if (p) return p;

        let res = await fetchPut<ClientAccessRulesResponse>(
            `/auth/v1/clients/${clientId}/access_rules`,
            { rules: rows.map(r => toRule(r, kindOf(r.field))) },
        );
        if (res.error) return res.error.message;
        if (res.body) rows = res.body.rules.map(toRow);
        return '';
    }

    function add() {
        let first = fieldOf('country') ?? fields[0];
        if (first) rows = [...rows, emptyRow(first.name, first.kind)];
    }

    function remove(i: number) {
        rows = rows.filter((_, idx) => idx !== i);
    }

    function pickField(i: number, field: string) {
        rows[i] = changeField(rows[i], field, kindOf(field));
    }

    function pickOp(i: number, op: string) {
        rows[i] = changeOp(rows[i], op, kindOf(rows[i].field));
    }

    /** a number input binds a number, the row keeps text */
    const asText = (v: unknown) => (v === null || v === undefined ? '' : String(v));
</script>

<FormSection
    title="Access rules"
    desc="Only users who meet <b>all</b> of these rules can log in to this client, for example <code>country is one of UG, KE, TZ</code> or <code>age is at least 18</code>. The rules are checked at every login and token refresh. A user who has no value for a field does not meet a rule on it."
>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    {#each rows as r, i (i)}
        {@const kind = kindOf(r.field)}
        {@const mode = valueMode(kind, r.op)}
        <div class="grid items-end gap-2 sm:grid-cols-[1.3fr_1fr_1.8fr_auto]">
            <InputSelect
                value={r.field}
                onChange={v => pickField(i, v)}
                label={i === 0 ? 'Field' : ''}
                items={fields.map(f => ({ value: f.name, label: fieldLabel(f) }))}
            />
            <InputSelect
                value={r.op}
                onChange={v => pickOp(i, v)}
                label={i === 0 ? 'Condition' : ''}
                items={opsFor(kind)}
            />

            {#if mode === 'none'}
                <div></div>
            {:else if mode === 'bool'}
                <InputSelect
                    bind:value={r.text}
                    label={i === 0 ? 'Value' : ''}
                    items={[
                        { value: 'true', label: 'true' },
                        { value: 'false', label: 'false' },
                    ]}
                />
            {:else if mode === 'range'}
                <div class="grid grid-cols-2 gap-2">
                    <Input
                        typ="number"
                        step="any"
                        bind:value={() => r.text, v => (r.text = asText(v))}
                        label={i === 0 ? 'From' : ''}
                        placeholder="lowest"
                        autocomplete="off"
                    />
                    <Input
                        typ="number"
                        step="any"
                        bind:value={() => r.text2, v => (r.text2 = asText(v))}
                        label={i === 0 ? 'To' : ''}
                        placeholder="highest"
                        autocomplete="off"
                    />
                </div>
            {:else if mode === 'number'}
                <Input
                    typ="number"
                    step="any"
                    bind:value={() => r.text, v => (r.text = asText(v))}
                    label={i === 0 ? 'Value' : ''}
                    placeholder="a number"
                    autocomplete="off"
                />
            {:else if mode === 'date'}
                <InputDate
                    bind:value={r.text}
                    label={i === 0 ? 'Value' : ''}
                    min="1900-01-01"
                    max="2100-01-01"
                />
            {:else if mode === 'many'}
                {#if kind === 'country'}
                    <MultiPick
                        bind:value={r.list}
                        items={countries}
                        label={i === 0 ? 'Value' : ''}
                        placeholder="pick countries"
                    />
                {:else if kind === 'choice' || kind === 'list'}
                    <MultiPick
                        bind:value={r.list}
                        items={optionItems(r.field)}
                        label={i === 0 ? 'Value' : ''}
                        placeholder="pick values"
                    />
                {:else}
                    <MultiPick
                        bind:value={r.list}
                        label={i === 0 ? 'Value' : ''}
                        placeholder="type a value, press Enter"
                    />
                {/if}
            {:else if kind === 'country'}
                <InputCountry bind:value={r.text} label={i === 0 ? 'Value' : ''} placeholder="pick a country" />
            {:else if kind === 'choice' || kind === 'list'}
                <InputSelect
                    bind:value={r.text}
                    label={i === 0 ? 'Value' : ''}
                    items={optionItems(r.field)}
                />
            {:else}
                <Input bind:value={r.text} label={i === 0 ? 'Value' : ''} autocomplete="off" />
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
