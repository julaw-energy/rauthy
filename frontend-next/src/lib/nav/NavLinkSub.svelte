<script lang="ts">
    import { page } from '$app/state';
    import type { Snippet } from 'svelte';
    import { useTrigger } from '$state/callback.svelte';
    import { cn } from '$lib/utils';

    let {
        ref = $bindable(),
        href,
        highlightWithParams = false,
        children,
    }: {
        ref?: undefined | HTMLAnchorElement;
        href: string;
        highlightWithParams?: boolean;
        children: Snippet;
    } = $props();

    let tr = useTrigger();
    let current = $derived(
        highlightWithParams
            ? `${page.route.id}${page.url.search}`.startsWith(href)
            : page.route.id === href.split('?')[0],
    );

    function onclick() {
        requestAnimationFrame(() => setTimeout(() => tr.trigger('navSub'), 100));
    }
</script>

<a
    bind:this={ref}
    {href}
    {onclick}
    aria-current={current ? 'page' : undefined}
    class={cn(
        'hover:bg-accent whitespace-nowrap rounded-xl px-3 py-2 text-sm transition-colors',
        current && 'bg-accent text-accent-foreground font-medium',
    )}
>
    {@render children()}
</a>
