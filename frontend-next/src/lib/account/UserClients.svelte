<script lang="ts">
    import { useI18n } from '$state/i18n.svelte.js';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import type { UserConsentDeleteResponse, UserConsentResponse } from '$api/types/user';
    import Button from '$lib5/button/Button.svelte';
    import ErrorAlert from '$lib/ErrorAlert.svelte';
    import { formatIsoDate } from '$lib5/dateFormat.svelte';

    interface UserClientResponse {
        client_id: string;
        client_name?: string;
        first_at: number;
        last_at: number;
        scope?: string;
        logins: number;
    }

    let { userId }: { userId: string } = $props();

    let t = useI18n();

    let clients: UserClientResponse[] = $state([]);
    // Recorded consents. Empty as long as `consent.enable` is off on the server.
    let consents: UserConsentResponse[] = $state([]);
    let loaded = $state(false);
    let err = $state('');
    let revoking = $state(false);
    // `client_id` + resource of the consent that waits for the second click
    let confirming = $state('');

    $effect(() => {
        load(userId);
    });

    async function load(id: string) {
        let res = await fetchGet<UserClientResponse[]>(`/auth/v1/users/${id}/clients`);
        if (res.body) {
            clients = res.body;
        } else {
            console.error('error fetching connected apps: ' + res.error);
        }
        await loadConsents(id);
        loaded = true;
    }

    async function loadConsents(id: string) {
        let res = await fetchGet<UserConsentResponse[]>(`/auth/v1/users/${id}/consents`);
        if (res.body) {
            consents = res.body;
        } else {
            // the list is a bonus, the page works without it
            consents = [];
        }
    }

    const consentKey = (c: UserConsentResponse) => JSON.stringify([c.client_id, c.resource ?? null]);

    // an app that has a consent but no login record still has to be listed, so it can be revoked
    let orphanConsents = $derived(
        consents.filter(c => !clients.some(cl => cl.client_id === c.client_id)),
    );

    async function revoke(c: UserConsentResponse) {
        if (confirming !== consentKey(c)) {
            // a revoke removes the refresh tokens of the app: ask once more
            confirming = consentKey(c);
            return;
        }
        revoking = true;
        err = '';
        let params = new URLSearchParams({ client_id: c.client_id });
        if (c.resource) {
            params.set('resource', c.resource);
        }
        let res = await fetchDelete<UserConsentDeleteResponse>(
            `/auth/v1/users/${userId}/consents?${params}`,
        );
        if (res.error) {
            err = res.error.message;
        } else {
            confirming = '';
            await loadConsents(userId);
        }
        revoking = false;
    }

    function fmt(ts: number) {
        let d = new Date(ts * 1000);
        let p = (n: number) => String(n).padStart(2, '0');
        let iso = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
        return `${formatIsoDate(iso)} ${p(d.getHours())}:${p(d.getMinutes())}`;
    }
</script>

{#snippet consentBlock(list: UserConsentResponse[])}
    {#each list as c (consentKey(c))}
        <div class="bg-muted/40 grid gap-1.5 rounded-lg border px-3 py-2 text-sm">
            <div class="text-muted-foreground grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
                {#if c.resource}
                    <span>{t.account.privacy.resource}</span>
                    <span class="text-foreground font-mono break-all">{c.resource}</span>
                {/if}
                <span>{t.account.privacy.scopes}</span>
                <span class="text-foreground font-mono break-all">{c.scopes.join(' ')}</span>
                <span>{t.account.privacy.granted}</span>
                <span class="text-foreground">{fmt(c.created)}</span>
                {#if c.updated !== c.created}
                    <span>{t.account.privacy.updated}</span>
                    <span class="text-foreground">{fmt(c.updated)}</span>
                {/if}
            </div>
            <div class="flex flex-wrap items-center gap-2">
                <Button level={-2} isLoading={revoking} onclick={() => revoke(c)}>
                    {confirming === consentKey(c) ? '✓ ' : ''}{t.account.privacy.revoke}
                </Button>
                {#if confirming === consentKey(c)}
                    <Button level={3} onclick={() => (confirming = '')}>{t.common.cancel}</Button>
                {/if}
            </div>
            {#if confirming === consentKey(c)}
                <p class="text-muted-foreground text-xs">{t.account.privacy.revokeDesc}</p>
            {/if}
        </div>
    {/each}
{/snippet}

<div class="grid max-w-3xl gap-3">
    <p class="text-muted-foreground text-sm">{t.account.connectedAppsDesc}</p>

    {#each clients as c (c.client_id)}
        <div class="grid gap-2 rounded-xl border p-3 text-sm">
            <div class="flex flex-wrap items-baseline justify-between gap-x-3">
                <span class="font-semibold">{c.client_name || c.client_id}</span>
                {#if c.client_name}
                    <span class="text-muted-foreground font-mono text-xs">{c.client_id}</span>
                {/if}
            </div>
            <div class="text-muted-foreground grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
                <span>{t.account.lastAccess}</span><span class="text-foreground">{fmt(c.last_at)}</span>
                <span>{t.account.firstAccess}</span><span class="text-foreground">{fmt(c.first_at)}</span>
                <span>{t.account.logins}</span><span class="text-foreground">{c.logins}</span>
            </div>
            {#if c.scope}
                <div class="flex flex-wrap gap-1">
                    {#each c.scope.split(/[\s,]+/).filter(Boolean) as s (s)}
                        <span class="bg-muted rounded-md px-1.5 py-0.5 font-mono text-xs">{s}</span>
                    {/each}
                </div>
            {/if}
            {@render consentBlock(consents.filter(x => x.client_id === c.client_id))}
        </div>
    {:else}
        {#if loaded && orphanConsents.length === 0}
            <p class="text-muted-foreground text-sm">—</p>
        {/if}
    {/each}

    {#each orphanConsents as c (consentKey(c))}
        <div class="grid gap-2 rounded-xl border p-3 text-sm">
            <span class="font-semibold">{c.client_name || c.client_id}</span>
            {@render consentBlock([c])}
        </div>
    {/each}

    <ErrorAlert {err} />
</div>
