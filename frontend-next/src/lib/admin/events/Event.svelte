<script lang="ts">
    import { onMount } from 'svelte';
    import { eventColor, formatDateFromTs } from '$utils/helpers';
    import type { EventResponse } from '$api/types/events.ts';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import A from '$lib5/A.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Tooltip from '$lib5/Tooltip.svelte';

    let {
        event,
        inline,
    }: {
        event: EventResponse;
        inline?: boolean;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let highlight = $state(false);
    let tooltip = $state(t.common.copyToClip);

    onMount(() => {
        let now = new Date().getTime();
        // this will only flash the event if it is not older than 30 seconds
        if (now - 1000 * 30 < event.timestamp) {
            setTimeout(() => (highlight = true), 100);
            setTimeout(() => (highlight = false), 2500);
        }
    });

    function copyToClip(v: string) {
        navigator.clipboard.writeText(v);
        tooltip = ta.common.copiedToClip;
        setTimeout(() => (tooltip = t.common.copyToClip), 3000);
    }
</script>

<div
    role="contentinfo"
    aria-label={`Event Level: ${event.level}}`}
    class="rounded-md border-l-4 px-3 py-2 text-sm transition-colors duration-700 {highlight ? 'bg-accent' : 'bg-muted/40'} {inline ? 'flex flex-wrap items-center gap-x-3' : ''}"
    style:border-left-color={eventColor(event.level)}
>
    <div aria-label="Timestamp" class="text-muted-foreground text-xs">
        {formatDateFromTs(event.timestamp / 1000)}
    </div>
    <div class="min-w-0 break-words">
        {#if event.typ !== 'RauthyHealthy' && event.typ !== 'RauthyUnhealthy' && event.typ !== 'NewRauthyVersion' && event.typ !== 'Test'}
            <span class="font-medium">{event.typ}</span>
        {/if}
        {#if event.typ === 'InvalidLogins'}
            {`: ${event.data}`}
        {:else if event.typ === 'NewRauthyVersion'}
            <A href={event.text || ''} target="_blank">New Version</A>
        {:else if event.typ === 'IpBlacklisted'}
            {ta.common.until}
            {event.data && formatDateFromTs(event.data)}
        {:else if event.text}
            {event.text}
        {/if}
    </div>
    {#if event.ip}
        <Button ariaLabel="IP Address" invisible onclick={() => copyToClip(event.ip || '')}>
            <Tooltip text={tooltip}>
                <span class="font-mono text-xs">{event.ip}</span>
            </Tooltip>
        </Button>
    {/if}
</div>
