<script lang="ts">
    import type { Snippet } from 'svelte';
    import { page } from '$app/state';
    import * as Sidebar from '$lib/components/ui/sidebar';
    import { useTrigger } from '$state/callback.svelte';

    let {
        route,
        icon,
        highlightIncludes,
        params = '',
        children,
    }: {
        compact?: boolean;
        params?: string;
        route: string;
        icon: Snippet<[string]>;
        highlightIncludes?: string;
        children: Snippet;
    } = $props();

    const urlPrefix = '/auth/v1/admin';
    const sidebar = Sidebar.useSidebar();
    let tr = useTrigger();

    let href = $derived(`${urlPrefix}${route}${params}`);
    let active = $derived(
        highlightIncludes ? page.url.pathname.includes(highlightIncludes) : page.url.pathname.startsWith(`${urlPrefix}${route}`),
    );

    function onclick() {
        if (sidebar.isMobile) sidebar.setOpenMobile(false);
        // the focus trigger is set by the page we navigate to -> wait until it exists
        requestAnimationFrame(() => setTimeout(() => tr.trigger('navMain'), 100));
    }
</script>

<Sidebar.MenuItem>
    <!-- the default active state (`sidebar-accent`) is only a few % off the background, so use the primary color -->
    <Sidebar.MenuButton
        isActive={active}
        tooltipContent={undefined}
        class="data-active:bg-primary data-active:text-primary-foreground data-active:hover:bg-primary data-active:hover:text-primary-foreground data-active:font-medium"
    >
        {#snippet child({ props })}
            <a {...props} {href} {onclick}>
                {@render icon('1.1rem')}
                <span>{@render children()}</span>
            </a>
        {/snippet}
    </Sidebar.MenuButton>
</Sidebar.MenuItem>
