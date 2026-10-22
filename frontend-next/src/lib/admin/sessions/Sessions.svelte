<script lang="ts">
    import { onMount } from 'svelte';
    import { redirectToLogin } from '$utils/helpers';
    import type { SessionResponse } from '$api/types/session.ts';
    import { fetchDelete } from '$api/fetch';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useSession } from '$state/session.svelte';
    import { fetchSearchServer, type SearchParamsIdxSession } from '$utils/search';
    import { useTrigger } from '$state/callback.svelte';
    import { Search } from '@lucide/svelte';
    import { Input } from '$lib/components/ui/input';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { boolCol, dateCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import { createSsp } from '$lib5/ssp.svelte';
    import Button from '$lib5/button/Button.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import Options from '$lib5/Options.svelte';

    let t = useI18n();
    let ta = useI18nAdmin();

    let session = useSession('admin');
    // session deletion is a full-admin action; group admins log a user out from the user
    // details tab instead, so these buttons are hidden for them
    let isAdmin = $derived(session.isAdmin());

    let refOpts: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refOpts?.focus());

    let err = $state('');
    const ssp = createSsp<SessionResponse>('/auth/v1/sessions', { onError: m => (err = m) });
    let isSearchedServer = $state(false);

    const searchOptions = ['User ID', 'Session ID', 'IP'];
    let searchOption = $state(searchOptions[0]);
    let searchValue = $state('');

    const columns: FeaturedColumnDef<SessionResponse>[] = [
        textCol<SessionResponse>('user_id', 'User ID', { size: 260 }),
        textCol<SessionResponse>('id', 'Session ID', { size: 260 }),
        textCol<SessionResponse>('state', ta.options.state, { size: 130 }),
        dateCol<SessionResponse>('exp', ta.options.expires),
        dateCol<SessionResponse>('last_seen', ta.options.lastSeen),
        textCol<SessionResponse>('remote_ip', 'IP', { size: 160 }),
        boolCol<SessionResponse>('is_mfa', 'MFA'),
    ];

    let sessionsShown = $derived.by(() => {
        if (ssp.active) return ssp.items;
        const q = searchValue.toLowerCase();
        if (!q) return ssp.items;
        if (searchOption === searchOptions[0]) return ssp.items.filter(s => s.user_id?.toLowerCase().includes(q));
        if (searchOption === searchOptions[1]) return ssp.items.filter(s => s.id.toLowerCase().includes(q));
        return ssp.items.filter(s => s.remote_ip?.toLowerCase().includes(q));
    });

    const remove: RowDeletion<SessionResponse> = {
        label: s => `session ${s.id}`,
        description: 'The user will be logged out of this session.',
        run: async s => {
            let res = await fetchDelete(`/auth/v1/sessions/id/${s.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: () => {
            // reload (trigger new login) if the delete session was ours
            ssp.load(ssp.active ? `page_size=${ssp.pageSize}` : undefined);
        },
    };

    onMount(() => {
        ssp.load('page_size=' + ssp.pageSize);
    });

    $effect(() => {
        let search = searchValue.toLowerCase();
        if (ssp.active) {
            if (search.length < 3) {
                if (isSearchedServer) {
                    ssp.first();
                    isSearchedServer = false;
                }
            } else {
                searchServer(search);
            }
        }
    });

    async function searchServer(q: string) {
        isSearchedServer = true;

        let idx: SearchParamsIdxSession;
        if (searchOption === searchOptions[0]) {
            idx = 'userid';
        } else if (searchOption === searchOptions[1]) {
            idx = 'sessionid';
        } else {
            idx = 'ip';
        }

        let res = await fetchSearchServer<SessionResponse[]>({ ty: 'session', idx, q });
        if (res.body) {
            ssp.items = res.body;
        } else {
            console.error(res.error);
        }
    }

    async function invalidateSessions() {
        let res = await fetchDelete('/auth/v1/sessions');
        if (res.error) {
            err = res.error.message;
        } else {
            redirectToLogin();
        }
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="sessions" class="min-h-0">
        <DataTable
            data={sessionsShown}
            {columns}
            clientSide={!ssp.active}
            paginationData={ssp.active ? ssp.paginationData : undefined}
            handleFetch={ssp.handleFetch}
            fetching={ssp.fetching}
            onFetch={() => ssp.load(ssp.active ? `page_size=${ssp.pageSize}` : undefined)}
            getRowId={s => s.id}
            remove={isAdmin ? remove : undefined}
            initialSorting={[{ id: 'last_seen', desc: true }]}
        >
            {#snippet toolbar()}
                <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
                    <div class="relative w-full min-w-48 sm:w-72">
                        <Search class="text-muted-foreground absolute top-1/2 left-2.5 size-4 -translate-y-1/2" />
                        <Input class="h-8 pl-8" placeholder={t.common.search} aria-label={t.common.search} bind:value={searchValue} />
                    </div>
                    <div class="w-32">
                        <Options bind:ref={refOpts} ariaLabel={t.common.search} options={searchOptions} bind:value={searchOption} />
                    </div>
                </div>
                {#if isAdmin}
                    <Button level={-1} onclick={invalidateSessions}>
                        {ta.sessions.invalidateAll}
                    </Button>
                {/if}
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>
