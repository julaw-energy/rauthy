<script lang="ts">
    import { useI18n } from '$state/i18n.svelte.js';
    import { fetchGet } from '$api/fetch';
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
    let loaded = $state(false);

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
        loaded = true;
    }

    function fmt(ts: number) {
        let d = new Date(ts * 1000);
        let p = (n: number) => String(n).padStart(2, '0');
        let iso = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
        return `${formatIsoDate(iso)} ${p(d.getHours())}:${p(d.getMinutes())}`;
    }
</script>

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
        </div>
    {:else}
        {#if loaded}
            <p class="text-muted-foreground text-sm">—</p>
        {/if}
    {/each}
</div>
