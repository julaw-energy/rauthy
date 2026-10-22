<script lang="ts">
    import type { Snippet } from 'svelte';

    // Secondary navigation: a vertical list on wide screens, a horizontal scroller on small ones.
    let {
        children,
        buttonTiles,
        buttonTilesAriaControls,
    }: {
        width?: string;
        paddingTop?: string;
        collapseButtonThreshold?: number;
        thresholdNavSub?: number;
        children: Snippet;
        buttonTiles?: Snippet;
        buttonTilesAriaControls?: string;
    } = $props();
</script>

<nav class="shrink-0 md:w-48">
    <div class="flex gap-1 overflow-x-auto pb-1 md:flex-col md:overflow-visible">
        {@render children()}
        {#if buttonTiles}
            <ul aria-controls={buttonTilesAriaControls} class="flex gap-1 md:flex-col">
                {@render buttonTiles()}
            </ul>
        {/if}
    </div>
</nav>
