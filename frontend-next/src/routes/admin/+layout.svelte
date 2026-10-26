<script lang="ts">
    import type { Snippet } from 'svelte';
    import { page } from '$app/state';
    import { Bell, TriangleAlert } from '@lucide/svelte';
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
    import { fetchGet, fetchPut } from '$api/fetch';
    import type { SettingsResponse } from '$api/types/settings';
    import { initI18nAdmin, useI18nAdmin } from '$state/i18n_admin.svelte';
    import { brandI18n } from '$lib5/appName.svelte';
    import { unread } from '$lib5/admin/events/unread.svelte';
    import { onDestroy } from 'svelte';

    let { children }: { children: Snippet } = $props();

    initI18nAdmin();
    brandI18n(useI18nAdmin());

    let ta = useI18nAdmin();
    let session = useSession('admin');

    let isAdmin = $state(false);
    // first-run hint: without an SMTP server no emails (password resets, codes) go out
    let smtpMissing = $state(false);
    let needsAdminRole = $state(false);
    let mfaReqErr = $state(false);
    let showEvents = $state(false);
    let innerWidth: undefined | number = $state();
    let wide = $derived(!!innerWidth && innerWidth >= 1536);

    // The bell. A light stream of the events the admin cares about (the level of the list) is used
    // to count what is newer than the read marker. The marker lives in the database, per admin, and
    // the stream sends it as the named event `read`, again whenever another tab or device moves it,
    // so the number is the same everywhere.
    let eventsWatch: undefined | EventSource;
    let listVisible = $derived(wide || showEvents || page.url.pathname.endsWith('/events'));
    // timestamps (ms) of the events received, newest last; only the number newer than the marker counts
    let seen: number[] = $state([]);
    let readUntil = $state(0);
    // what was last sent to the server, so a marker is not sent twice
    let sentUntil = 0;

    $effect(() => {
        unread.count = listVisible ? 0 : seen.filter(ts => ts > readUntil).length;
    });

    // opening the list marks everything seen so far as read, and so does every event that arrives
    // while it is open
    $effect(() => {
        if (listVisible && seen.length > 0) markRead(Math.max(...seen));
    });

    async function markRead(until: number) {
        if (until <= readUntil || until <= sentUntil) return;
        sentUntil = until;
        let res = await fetchPut<{ read_until: number }>('/auth/v1/events/read', { read_until: until });
        if (res.body) applyMarker(res.body.read_until);
    }

    function applyMarker(until: number) {
        // it only moves forward
        if (until > readUntil) readUntil = until;
    }

    async function syncMarker() {
        let res = await fetchGet<{ read_until: number }>('/auth/v1/events/read');
        if (res.body) applyMarker(res.body.read_until);
    }

    $effect(() => {
        if (!isAdmin || eventsWatch) return;
        // the level picked in the event list, so the bell counts what the list would show
        let level = 'info';
        try {
            level = localStorage.getItem('eventLevel')?.toLowerCase() || 'info';
        } catch {
            // storage is not available, the default is fine
        }
        const es = new EventSource(`/auth/v1/events/stream?latest=50&level=${level}&read=true`);
        es.addEventListener('read', ev => {
            try {
                applyMarker(JSON.parse((ev as MessageEvent).data).read_until);
            } catch {
                // not a marker we understand
            }
        });
        es.onmessage = ev => {
            if (!ev.data) return;
            try {
                const ts = JSON.parse(ev.data).timestamp;
                if (typeof ts === 'number') seen = [...seen.slice(-499), ts];
            } catch {
                // not an event
            }
        };
        // An admin who never read anything starts with a clean bell instead of the replayed backlog.
        es.onopen = () => {
            setTimeout(() => {
                if (readUntil === 0) markRead(seen.length ? Math.max(...seen) : Date.now());
            }, 1500);
        };
        eventsWatch = es;
    });

    // another node of a cluster does not push the marker, so ask again when the tab is looked at
    function onVisible() {
        if (document.visibilityState === 'visible' && isAdmin) syncMarker();
    }

    onDestroy(() => eventsWatch?.close());

    $effect(() => {
        let s = session.get();
        if (s) {
            // full Rauthy admins and delegated group admins both reach the admin UI;
            // the backend still scopes every action, and the nav hides what group admins cannot do
            if (session.isAnyAdmin()) {
                isAdmin = true;
                // async check for admin access speeds up FCP and is still fast enough for a good UX
                checkAdminAccess();
                if (session.isAdmin()) checkSetup();
            } else {
                needsAdminRole = true;
            }
        }
    });

    async function checkSetup() {
        let res = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (res.body) smtpMissing = !res.body.smtp_configured;
    }

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

<svelte:window bind:innerWidth onfocus={onVisible} />
<svelte:document onvisibilitychange={onVisible} />

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
                            <span class="relative inline-flex">
                                <Bell class="size-5" />
                                {#if unread.count > 0}
                                    <span
                                        class="bg-destructive text-destructive-foreground absolute -top-2 -right-2 flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[10px] leading-none font-semibold"
                                        aria-label={`${unread.count} new events`}
                                    >
                                        {unread.count > 99 ? '99+' : unread.count}
                                    </span>
                                {/if}
                            </span>
                        </Button>
                    {/if}
                </div>
            </header>

            <div class="flex min-h-0 flex-1 gap-4 p-3 sm:p-4">
                <div class="flex min-w-0 flex-1 flex-col gap-4">
                    {#if smtpMissing && !page.url.pathname.endsWith('/config/email')}
                        <div
                            class="flex flex-wrap items-center gap-3 rounded-xl border border-amber-500/50 bg-amber-500/10 px-4 py-3 text-sm"
                            role="status"
                        >
                            <TriangleAlert class="size-4 shrink-0 text-amber-600 dark:text-amber-400" />
                            <span class="flex-1">
                                No mail server is set up yet, so Rauthy cannot send emails (password resets,
                                verification codes).
                            </span>
                            <A href="/auth/v1/admin/config/email">Set up email</A>
                        </div>
                    {/if}
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
