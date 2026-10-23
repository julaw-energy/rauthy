<script lang="ts">
    import type { Snippet } from 'svelte';
    import { page } from '$app/state';
    import { Bell } from '@lucide/svelte';
    import * as Sidebar from '$lib/components/ui/sidebar';
    import * as Sheet from '$lib/components/ui/sheet';
    import { Separator } from '$lib/components/ui/separator';
    import Button from '$lib5/button/Button.svelte';
    import NavSide from '$lib5/nav/NavSide.svelte';
    import Events from '$lib5/admin/events/Events.svelte';
    import Loading from '$lib5/Loading.svelte';
    import DateFormatSelect from '$lib5/DateFormatSelect.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import LangSelector from '$lib5/LangSelector.svelte';
    import A from '$lib5/A.svelte';
    import Tooltip from '$lib5/Tooltip.svelte';
    import IconUserCircle from '$icons/IconUserCircle.svelte';
    import IconLogout from '$icons/IconLogout.svelte';
    import { redirectToLogout } from '$utils/helpers';
    import { useSession } from '$state/session.svelte';
    import { fetchGet } from '$api/fetch';
    import { initI18nAdmin, useI18nAdmin } from '$state/i18n_admin.svelte';

    let { children }: { children: Snippet } = $props();

    initI18nAdmin();

    let ta = useI18nAdmin();
    let session = useSession('admin');

    let isAdmin = $state(false);
    let needsAdminRole = $state(false);
    let mfaReqErr = $state(false);
    let showEvents = $state(false);
    let innerWidth: undefined | number = $state();
    let wide = $derived(!!innerWidth && innerWidth >= 1536);

    $effect(() => {
        let s = session.get();
        if (s) {
            // full Rauthy admins and delegated group admins both reach the admin UI;
            // the backend still scopes every action, and the nav hides what group admins cannot do
            if (session.isAnyAdmin()) {
                isAdmin = true;
                // async check for admin access speeds up FCP and is still fast enough for a good UX
                checkAdminAccess();
            } else {
                needsAdminRole = true;
            }
        }
    });

    async function checkAdminAccess() {
        let res = await fetchGet('/auth/v1/auth_check_admin');
        if (res.status === 406) {
            mfaReqErr = true;
        }
    }

    const titles: Record<string, string> = {
        users: 'users',
        clients: 'clients',
        roles: 'roles',
        groups: 'groups',
        attributes: 'attributes',
        scopes: 'scopes',
        sessions: 'sessions',
        events: 'events',
        blacklist: 'blacklist',
        api_keys: 'apiKeys',
        providers: 'providers',
        config: 'config',
        docs: 'docs',
    };
    let section = $derived(page.url.pathname.split('/')[4] ?? 'users');
    // sections without a translated nav label
    const fixedTitles: Record<string, string> = { kv: 'KV Store', pam: 'PAM' };
    let title = $derived(fixedTitles[section] ?? (ta.nav as Record<string, string>)[titles[section] ?? section] ?? section);
</script>

<svelte:window bind:innerWidth />

<svelte:head>
    <title>Rauthy Admin</title>
</svelte:head>

{#if mfaReqErr || needsAdminRole}
    <div class="flex min-h-dvh flex-col items-center justify-center gap-4 p-6 text-center">
        <div class="max-w-md">
            {#if mfaReqErr}
                {@html ta.error.noAdmin}
            {:else}
                {@html ta.error.needsAdminRole}
            {/if}
        </div>
        {#if mfaReqErr}
            <Button onclick={() => (window.location.href = '/auth/v1/account')}>{ta.common.account}</Button>
        {:else}
            <Button onclick={() => (window.location.href = '/auth/v1/')}>{ta.common.back}</Button>
        {/if}
    </div>
{:else if isAdmin}
    <Sidebar.Provider>
        <NavSide />
        <Sidebar.Inset class="min-w-0">
            <header class="bg-background/80 sticky top-0 z-20 flex h-14 shrink-0 items-center gap-2 border-b px-3 backdrop-blur">
                <Sidebar.Trigger />
                <Separator orientation="vertical" class="h-5" />
                <h1 class="min-w-0 flex-1 truncate text-base font-semibold capitalize">{title}</h1>
                <!-- same controls, same order as the account page; icon buttons share one 36px box so they line up -->
                <div
                    class="flex shrink-0 items-center gap-1 [&_button:not([data-slot=select-trigger])]:size-9! [&_button:not([data-slot=select-trigger])]:p-0!"
                >
                    <DateFormatSelect />
                    <ThemeSwitch />
                    <div class="w-24 sm:w-28"><LangSelector borderless={false} /></div>
                    <A href="/auth/v1/account" hideUnderline>
                        <span class="inline-flex size-9 items-center justify-center">
                            <Tooltip text="Account"><IconUserCircle width="1.4rem" /></Tooltip>
                        </span>
                    </A>
                    <Button invisible ariaLabel="Logout" onclick={() => redirectToLogout()}>
                        <Tooltip text="Logout"><IconLogout width="1.4rem" /></Tooltip>
                    </Button>
                    {#if !wide}
                        <Button invisible ariaLabel="Events" onclick={() => (showEvents = true)}>
                            <Bell class="size-5" />
                        </Button>
                    {/if}
                </div>
            </header>

            <div class="flex min-h-0 flex-1 gap-4 p-3 sm:p-4">
                <div class="flex min-w-0 flex-1 flex-col gap-4">
                    {@render children()}
                </div>
                {#if wide}
                    <aside class="sticky top-[4.5rem] hidden h-[calc(100dvh-6rem)] w-80 shrink-0 2xl:block">
                        <Events />
                    </aside>
                {/if}
            </div>
        </Sidebar.Inset>

        {#if !wide}
            <Sheet.Root bind:open={showEvents}>
                <Sheet.Content side="right" class="w-full max-w-md p-3 sm:max-w-md">
                    <Sheet.Header class="sr-only">
                        <Sheet.Title>Events</Sheet.Title>
                    </Sheet.Header>
                    {#if showEvents}
                        <div class="h-full pt-8"><Events /></div>
                    {/if}
                </Sheet.Content>
            </Sheet.Root>
        {/if}
    </Sidebar.Provider>
{:else}
    <div class="flex min-h-dvh items-center justify-center"><Loading /></div>
{/if}
