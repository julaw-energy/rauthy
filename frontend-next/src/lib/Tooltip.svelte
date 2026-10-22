<script lang="ts">
    import type { Snippet } from 'svelte';
    import * as Tooltip from '$lib/components/ui/tooltip';

    let {
        text,
        children,
    }: {
        xOffset?: number;
        yOffset?: number;
        text: string;
        children: Snippet;
    } = $props();
</script>

<Tooltip.Provider>
    <Tooltip.Root>
        <Tooltip.Trigger>
            {#snippet child({ props })}
                <span {...props} class="inline-flex">{@render children()}</span>
            {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content>{text}</Tooltip.Content>
    </Tooltip.Root>
</Tooltip.Provider>
