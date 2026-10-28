<script lang="ts">
    import { fetchGet } from '$api/fetch';
    import type { AuditSummary } from '$api/types/events';

    // "Created by ... / last changed by ..." for one user, client, ... Read from the audit log, so
    // what existed before it has no creator. Nothing is shown when it cannot be read (a group admin
    // has no access to the audit log) or when there is nothing to show.
    let {
        targetId,
        refreshKey,
    }: {
        targetId: string;
        /** anything that changes when the thing was saved, to read the log again */
        refreshKey?: unknown;
    } = $props();

    let s: AuditSummary | undefined = $state();

    $effect(() => {
        void refreshKey;
        const id = targetId;
        s = undefined;
        // the log is written in the background, give the save a moment
        const timer = setTimeout(async () => {
            const res = await fetchGet<AuditSummary>(
                `/auth/v1/events/audit/summary?target_id=${encodeURIComponent(id)}`,
                'json',
                'noRedirect',
            );
            if (id === targetId) s = res.body;
        }, 400);
        return () => clearTimeout(timer);
    });

    const when = (ms?: number | null) => (ms ? new Date(ms).toLocaleString() : '');
    const who = (id?: string | null, label?: string | null) => label || id || '';
</script>

{#if s && (s.created_by || s.modified_by)}
    <p class="text-muted-foreground mt-2 text-xs">
        {#if s.created_by}
            Created by <span title={s.created_by}>{who(s.created_by, s.created_by_label)}</span>
            on {when(s.created_at)}.
        {/if}
        {#if s.modified_by}
            Last change by <span title={s.modified_by}>{who(s.modified_by, s.modified_by_label)}</span>
            on {when(s.modified_at)}.
        {/if}
    </p>
{/if}
