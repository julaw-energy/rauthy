<script lang="ts">
    import type { Snippet } from 'svelte';
    import { page } from '$app/state';
    import { cn } from '$lib/utils';

    let {
        ref = $bindable(),
        href,
        target,
        selectedStep = false,
        hideUnderline = false,
        highlight = false,
        highlightExact = false,
        highlightIncludes,
        highlightWithParams = false,
        onclick,
        children,
    }: {
        ref?: undefined | HTMLAnchorElement;
        href: string;
        target?: string;
        selectedStep?: boolean;
        hideUnderline?: boolean;
        highlight?: boolean;
        highlightExact?: boolean;
        highlightIncludes?: string;
        highlightWithParams?: boolean;
        onclick?: () => void;
        children: Snippet;
    } = $props();

    let active = $derived.by(() => {
        if (!highlight && !highlightExact && !highlightIncludes && !selectedStep) return false;
        const p = page.url.pathname;
        if (highlightExact) return p === href;
        if (highlightIncludes) return p.includes(highlightIncludes);
        if (selectedStep) return true;
        return p.startsWith(href);
    });
</script>

<a
    bind:this={ref}
    {href}
    {target}
    rel={target === '_blank' ? 'noopener noreferrer' : undefined}
    {onclick}
    class={cn('text-primary underline-offset-4', hideUnderline ? 'no-underline' : 'hover:underline', active && 'font-semibold')}
>
    {@render children()}
</a>
