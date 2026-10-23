<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type { LifetimesSettings, SettingsResponse } from '$api/types/settings';
    import Button from '$lib5/button/Button.svelte';
    import Switch from '$lib5/Switch.svelte';
    import Msg from '$lib5/Msg.svelte';
    import { Badge } from '$lib/components/ui/badge';
    import * as Select from '$lib/components/ui/select/index.js';

    type DurationKey = Exclude<keyof LifetimesSettings, 'session_renew_mfa'>;

    const rows: { key: DurationKey; label: string; hint: string }[] = [
        {
            key: 'session_lifetime',
            label: 'Session lifetime',
            hint: 'The longest a login stays valid, whatever the activity.',
        },
        {
            key: 'session_timeout',
            label: 'Session timeout',
            hint: 'A session without activity for this long ends. Cannot be longer than the lifetime.',
        },
        {
            key: 'refresh_token_lifetime',
            label: 'Refresh token lifetime',
            hint: 'How long apps can keep renewing their access without the user logging in again.',
        },
        {
            key: 'refresh_token_grace_time',
            label: 'Refresh token grace time',
            hint: 'A just-used refresh token still works this long, so parallel requests do not fail.',
        },
        {
            key: 'magic_link_pwd_reset',
            label: 'Password reset link',
            hint: 'How long the link in a password reset email works.',
        },
        {
            key: 'magic_link_pwd_first',
            label: 'First password link',
            hint: 'How long the link in the welcome email of a new user works.',
        },
    ];

    const units = [
        { label: 'seconds', secs: 1 },
        { label: 'minutes', secs: 60 },
        { label: 'hours', secs: 3600 },
        { label: 'days', secs: 86400 },
    ];

    // each row edits a number in a unit; the unit that divides the value evenly and is biggest is shown first
    let amounts: Record<string, string> = $state({});
    let unitIdx: Record<string, number> = $state({});
    let renewMfa = $state(false);
    let fromDb = $state(false);
    let loaded = $state(false);
    let busy = $state(false);
    let err = $state('');

    onMount(load);

    async function load() {
        let res = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (!res.body) {
            err = res.error?.message || 'Error';
            return;
        }
        const l = res.body.lifetimes;
        for (const r of rows) {
            const secs = l[r.key];
            let idx = 0;
            units.forEach((u, i) => {
                if (secs > 0 && secs % u.secs === 0) idx = i;
            });
            unitIdx[r.key] = idx;
            amounts[r.key] = String(secs / units[idx].secs);
        }
        renewMfa = l.session_renew_mfa;
        fromDb = l.from_db;
        loaded = true;
    }

    function toSeconds(key: DurationKey) {
        return Math.round(Number(amounts[key]) * units[unitIdx[key]].secs);
    }

    async function save() {
        err = '';
        const payload = { session_renew_mfa: renewMfa } as LifetimesSettings;
        for (const r of rows) {
            const secs = toSeconds(r.key);
            if (!Number.isFinite(secs) || secs < 0) {
                err = `${r.label}: enter a valid number`;
                return;
            }
            payload[r.key] = secs;
        }
        busy = true;
        let res = await fetchPut('/auth/v1/settings/lifetimes', payload);
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Saved. It applies to new sessions, tokens and links.');
        await load();
    }

    async function importFromFile() {
        err = '';
        busy = true;
        let res = await fetchPost('/auth/v1/settings/import/lifetimes');
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
        <h3 class="text-xl font-semibold">Sessions &amp; tokens</h3>
        {#if loaded}
            {#if fromDb}
                <Badge variant="secondary">Stored in the database</Badge>
            {:else}
                <Badge variant="outline">From the config file</Badge>
            {/if}
        {/if}
    </div>

    <p class="text-muted-foreground text-sm">
        How long logins, tokens and email links stay valid. Changes apply to everything created from now on;
        what already exists keeps the lifetime it was created with.
    </p>

    {#if loaded}
        <div class="bg-card grid gap-4 rounded-xl border p-4">
            {#each rows as r (r.key)}
                <div class="grid gap-1.5">
                    <label class="text-sm font-medium" for={`lt-${r.key}`}>{r.label}</label>
                    <div class="flex gap-2">
                        <input
                            id={`lt-${r.key}`}
                            type="number"
                            min="0"
                            step="any"
                            class="border-input bg-background h-9 w-28 rounded-md border px-3 text-sm"
                            bind:value={amounts[r.key]}
                        />
                        <Select.Root
                            type="single"
                            value={String(unitIdx[r.key])}
                            onValueChange={v => {
                                // keep the same amount of time when the unit changes
                                const secs = toSeconds(r.key);
                                unitIdx[r.key] = Number(v);
                                amounts[r.key] = String(+(secs / units[Number(v)].secs).toFixed(4));
                            }}
                        >
                            <Select.Trigger class="w-32">{units[unitIdx[r.key]].label}</Select.Trigger>
                            <Select.Content>
                                {#each units as u, i (u.label)}
                                    <Select.Item value={String(i)} label={u.label}>{u.label}</Select.Item>
                                {/each}
                            </Select.Content>
                        </Select.Root>
                    </div>
                    <p class="text-muted-foreground text-xs">{r.hint}</p>
                </div>
            {/each}

            <Switch bind:checked={renewMfa} ariaLabel="Ask for MFA when a session is renewed">
                Ask for MFA again when a session is renewed
            </Switch>
        </div>

        <div class="flex flex-wrap items-center gap-2">
            <Button onclick={save} isLoading={busy}>Save</Button>
            <Button level={3} onclick={importFromFile} isDisabled={busy}>Import from config file</Button>
        </div>
    {/if}
    <Msg {err} />
</div>
