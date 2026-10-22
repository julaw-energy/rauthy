<script lang="ts">
    import { eventColor } from '$utils/helpers';
    import { useI18n } from '$state/i18n.svelte';

    let { wide }: { wide?: boolean } = $props();

    let t = useI18n();
</script>

<div aria-hidden="true" class="text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-1 border-t pt-2 text-xs">
    <span class="font-medium">{t.common.legend}</span>
    {#each ['info', 'notice', 'warning', 'critical'] as lvl (lvl)}
        <span class="flex items-center gap-1">
            <span class="inline-block size-2.5 rounded-full" style:background={eventColor(lvl as 'info')}></span>
            <span class="capitalize">{lvl}</span>
        </span>
    {/each}
</div>
