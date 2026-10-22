<script lang="ts">
    import type { Snippet } from 'svelte';

    // A titled, bordered group of related settings inside a long form.
    let {
        title,
        desc,
        children,
    }: {
        title?: string;
        /** may contain HTML, it comes from the (trusted) i18n texts */
        desc?: string;
        children: Snippet;
    } = $props();
</script>

<section class="grid gap-3 rounded-xl border p-4">
    {#if title}
        <h4 class="text-base font-semibold">{title}</h4>
    {/if}
    {#if desc}
        <p class="text-muted-foreground text-sm">{@html desc}</p>
    {/if}
    {@render children()}
</section>
