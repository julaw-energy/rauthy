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
    // `page.route.id` has no base path (`/admin/config/jwks`) while the href has (`/auth/v1/...`),
    // so compare the real path instead
    let current = $derived(
        highlightWithParams
            ? `${page.url.pathname}${page.url.search}`.startsWith(href)
            : page.url.pathname.replace(/\/$/, '') === href.split('?')[0].replace(/\/$/, ''),
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
        current && 'bg-primary text-primary-foreground hover:bg-primary font-medium',
    )}
>
    {@render children()}
</a>
