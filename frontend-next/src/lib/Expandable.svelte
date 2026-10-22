<script lang="ts">
    import type { Snippet } from 'svelte';
    import { ChevronRight } from '@lucide/svelte';
    import * as Collapsible from '$lib/components/ui/collapsible';
    import { useI18n } from '$state/i18n.svelte';

    let {
        expand = $bindable(false),
        summary,
        details,
    }: {
        expand?: boolean;
        summary: Snippet;
        details: Snippet;
    } = $props();

    let t = useI18n();
</script>

<Collapsible.Root bind:open={expand} class="w-full">
    <Collapsible.Trigger
        class="hover:bg-accent flex w-full items-center gap-2 rounded-xl px-2 py-1.5 text-left text-sm"
        aria-label={t.common.expandContent}
    >
        <ChevronRight class="size-4 shrink-0 transition-transform {expand ? 'rotate-90' : ''}" />
        <span class="font-medium">{@render summary()}</span>
    </Collapsible.Trigger>
    <Collapsible.Content class="px-2 pt-2 pb-1 text-sm">
        {@render details()}
    </Collapsible.Content>
</Collapsible.Root>
