<script lang="ts">
    import type { Snippet } from 'svelte';
    import InfoTip from '$lib/InfoTip.svelte';

    // A titled, bordered group of related settings inside a long form.
    let {
        title,
        desc,
        children,
    }: {
        title?: string;
        /** may contain HTML, it comes from the (trusted) i18n texts. Shown in an (i) next to the title */
        desc?: string;
        children: Snippet;
    } = $props();
</script>

<section class="grid gap-3 rounded-xl border p-4">
    {#if title}
        <h4 class="flex items-center gap-1 text-base font-semibold">
            {title}
            {#if desc}<InfoTip html text={desc} label={`About ${title}`} />{/if}
        </h4>
    {:else if desc}
        <p class="text-muted-foreground text-sm">{@html desc}</p>
    {/if}
    {@render children()}
</section>
