<script lang="ts">
    import { fetchGet } from '$api/fetch';
    import type { AuditEntry, AuditSummary } from '$api/types/events';
    import Button from '$lib5/button/Button.svelte';

    // "Status Change history" of one user, client, provider, ...: who created it and when, who
    // changed it last and when, and every change in between. Read from the audit log. What was
    // created before the audit log existed has no creator on record.
    let { targetId }: { targetId: string } = $props();

    const PAGE = 30;

    let summary: AuditSummary | undefined = $state();
    let entries: AuditEntry[] = $state([]);
    let more = $state(false);
    let loading = $state(true);
    let err = $state('');

    async function load(older: boolean) {
        loading = true;
        err = '';
        const q = new URLSearchParams({ target_id: targetId, limit: String(PAGE) });
        if (older && entries.length > 0) q.set('before', String(entries[entries.length - 1].at));
        const res = await fetchGet<AuditEntry[]>(`/auth/v1/events/audit?${q}`);
        loading = false;
        if (!res.body) {
            err = res.error?.message || 'The history could not be loaded';
            return;
        }
        entries = older ? [...entries, ...res.body] : res.body;
        more = res.body.length === PAGE;
    }

    async function loadSummary() {
        const res = await fetchGet<AuditSummary>(
            `/auth/v1/events/audit/summary?target_id=${encodeURIComponent(targetId)}`,
        );
        summary = res.body;
    }

    $effect(() => {
        void targetId;
        entries = [];
        summary = undefined;
        loadSummary();
        load(false);
    });

    const when = (ms?: number | null) => (ms ? new Date(ms).toLocaleString() : '');
    const who = (id?: string | null, label?: string | null) => label || id || '';
    const actor = (e: AuditEntry) => who(e.actor, e.actor_label);
    const WHAT: Record<string, string> = {
        create: 'Created',
        update: 'Changed',
        delete: 'Deleted',
        action: 'Action',
    };
    const what = (a: string) => WHAT[a] ?? a.replaceAll('_', ' ');
    const tone = (a: string) =>
        a === 'delete' ? 'text-destructive' : a === 'create' ? 'text-primary' : '';
</script>

<div class="grid gap-4">
    <div class="grid gap-3 rounded-xl border p-4 text-sm sm:grid-cols-2">
        <div>
            <div class="text-muted-foreground text-xs uppercase">Created by</div>
            {#if summary?.created_by}
                <div class="font-medium" title={summary.created_by}>
                    {who(summary.created_by, summary.created_by_label)}
                </div>
                <div class="text-muted-foreground">{when(summary.created_at)}</div>
            {:else}
                <div class="text-muted-foreground">Not on record: it is older than the audit log.</div>
            {/if}
        </div>
        <div>
            <div class="text-muted-foreground text-xs uppercase">Last modified by</div>
            {#if summary?.modified_by}
                <div class="font-medium" title={summary.modified_by}>
                    {who(summary.modified_by, summary.modified_by_label)}
                </div>
                <div class="text-muted-foreground">{when(summary.modified_at)}</div>
            {:else}
                <div class="text-muted-foreground">No change on record yet.</div>
            {/if}
        </div>
    </div>

    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <ol class="grid gap-2">
        {#each entries as e (e.id)}
            <li class="rounded-lg border p-3 text-sm">
                <div class="flex flex-wrap items-baseline justify-between gap-x-3">
                    <span class="font-medium {tone(e.action)}">{what(e.action)}</span>
                    <span class="text-muted-foreground text-xs">{when(e.at)}</span>
                </div>
                <div title={e.actor}>by {actor(e)}</div>
                {#if e.details}
                    <div class="text-muted-foreground text-xs break-words">{e.details}</div>
                {/if}
            </li>
        {/each}
    </ol>

    {#if !loading && entries.length === 0 && !err}
        <p class="text-muted-foreground text-sm">Nothing recorded yet.</p>
    {/if}
    {#if more}
        <div>
            <Button level={2} onclick={() => load(true)}>Older entries</Button>
        </div>
    {/if}
</div>
