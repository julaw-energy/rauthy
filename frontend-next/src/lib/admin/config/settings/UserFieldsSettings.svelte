<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type {
        SettingsResponse,
        UserValueRequirement,
        UserValuesSettings,
    } from '$api/types/settings';
    import Button from '$lib5/button/Button.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import Switch from '$lib5/Switch.svelte';
    import Msg from '$lib5/Msg.svelte';
    import { Badge } from '$lib/components/ui/badge';

    type Field = Exclude<keyof UserValuesSettings, 'revalidate_during_login' | 'name_order_reverse'>;

    const fields: { key: Field; label: string; note?: string }[] = [
        { key: 'given_name', label: 'Given name', note: 'cannot be hidden' },
        { key: 'family_name', label: 'Family name' },
        { key: 'birthdate', label: 'Birthdate' },
        { key: 'gender', label: 'Gender' },
        { key: 'phone', label: 'Phone number' },
        { key: 'country', label: 'Country' },
        { key: 'tz', label: 'Timezone' },
        { key: 'street', label: 'Street' },
        { key: 'zip', label: 'ZIP / postal code' },
        { key: 'city', label: 'City' },
    ];

    const levels: { value: UserValueRequirement; label: string }[] = [
        { value: 'required', label: 'Required' },
        { value: 'optional', label: 'Optional' },
        { value: 'hidden', label: 'Hidden' },
    ];

    let values: UserValuesSettings | undefined = $state();
    let fromDb = $state(false);
    let busy = $state(false);
    let err = $state('');

    onMount(load);

    async function load() {
        let res = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (res.body) {
            const { from_db, ...v } = res.body.user_values;
            values = v;
            fromDb = from_db;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function save() {
        if (!values) return;
        err = '';
        busy = true;
        let res = await fetchPut('/auth/v1/settings/user_values', values);
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Saved. Open pages pick it up on their next load.');
        await load();
    }

    async function importFromFile() {
        err = '';
        busy = true;
        let res = await fetchPost('/auth/v1/settings/import/user_values');
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Imported from the config file');
        await load();
    }
</script>

<div class="grid max-w-2xl gap-4">
    <div class="flex flex-wrap items-center gap-3">
        <h3 class="text-xl font-semibold">User fields</h3>
        {#if values}
            {#if fromDb}
                <Badge variant="secondary">Stored in the database</Badge>
            {:else}
                <Badge variant="outline">From the config file</Badge>
            {/if}
        {/if}
    </div>

    <p class="text-muted-foreground text-sm">
        Which profile fields users see when they register or edit their account, and which of them they must
        fill in. Changes apply immediately. The preferred username rules (pattern, blocked names) stay in the
        config file.
    </p>

    {#if values}
        <div class="bg-card grid gap-1 rounded-xl border p-3">
            {#each fields as f (f.key)}
                <div class="grid items-center gap-2 py-1 sm:grid-cols-[1fr_11rem]">
                    <div>
                        <span class="text-sm font-medium">{f.label}</span>
                        {#if f.note}<span class="text-muted-foreground ml-2 text-xs">{f.note}</span>{/if}
                    </div>
                    <InputSelect
                        bind:value={() => values![f.key], v => (values![f.key] = v as UserValueRequirement)}
                        items={f.key === 'given_name' ? levels.filter(l => l.value !== 'hidden') : levels}
                        required
                    />
                </div>
            {/each}
        </div>

        <div class="grid gap-3">
            <Switch bind:checked={values.name_order_reverse} ariaLabel="Family name first">
                Show the family name before the given name
            </Switch>
            <Switch bind:checked={values.revalidate_during_login} ariaLabel="Revalidate during login">
                Ask users to complete missing required fields when they log in to a client
            </Switch>
        </div>

        <div class="flex flex-wrap items-center gap-2">
            <Button onclick={save} isLoading={busy}>Save</Button>
            <Button level={3} onclick={importFromFile} isDisabled={busy}>Import from config file</Button>
        </div>
    {/if}
    <Msg {err} />
</div>
