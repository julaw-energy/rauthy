<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import type { PaginationState } from '@tanstack/svelte-table';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import { useSession } from '$state/session.svelte';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import type { UserResponse, UserResponseSimple } from '$api/types/user.ts';
    import type { GroupResponse } from '$api/types/groups.ts';
    import type { RoleResponse } from '$api/types/roles.ts';
    import type { AuthProviderTemplate } from '$api/templates/AuthProvider';
    import type { UserValuesConfig } from '$api/templates/UserValuesConfig';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchSearchServer, type SearchParamsIdxUser } from '$utils/search';
    import { Input } from '$lib/components/ui/input';
    import { Search } from '@lucide/svelte';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { componentCol, dateCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import Options from '$lib5/Options.svelte';
    import SendMail from '$lib5/SendMail.svelte';
    import EmailJobs from '$lib5/EmailJobs.svelte';
    import UserAddNew from '$lib5/admin/users/UserAddNew.svelte';
    import UserDetails from '$lib5/admin/users/UserDetails.svelte';
    import UserCell from '$lib5/admin/users/UserCell.svelte';

    let t = useI18n();
    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);

    let closeModal: undefined | (() => void) = $state();
    let err = $state('');

    let userValuesConfig: undefined | UserValuesConfig = $state();
    let users: UserResponseSimple[] = $state([]);
    let uid = useParam('uid');
    let userId: undefined | string = $derived(uid.get());

    let providers: AuthProviderTemplate[] = $state([]);
    let groups: GroupResponse[] = $state([]);
    let roles: RoleResponse[] = $state([]);

    // server side pagination (SSP) kicks in for large installations (HTTP 206)
    let useServerSide = $state(false);
    let sspHeaders: undefined | Headers = $state();
    let sspPageSize = $state(20);
    let sspPage = $state(1);
    let sspToken: string | undefined | null = $state();
    let sspTotal = $state(0);
    let isSearchedServer = $state(false);
    let fetching = $state(false);

    const searchOptions = ['E-Mail', 'ID'];
    let searchOption = $state(searchOptions[0]);
    let searchValue = $state('');

    const columns: FeaturedColumnDef<UserResponseSimple>[] = [
        textCol<UserResponseSimple>('id', 'ID', { size: 230 }),
        componentCol<UserResponseSimple, Record<string, never>>('user', 'User', UserCell, undefined, {
            accessorFn: (u: UserResponseSimple) => `${u.email} ${u.given_name ?? ''} ${u.family_name ?? ''}`,
            enableColumnFilter: true,
            size: 320,
            meta: { dataType: 'text', filter: { type: 'text' } },
        }),
        dateCol<UserResponseSimple>('created_at', 'Created'),
        dateCol<UserResponseSimple>('last_login', 'Last Login'),
    ];

    let usersShown = $derived.by(() => {
        if (useServerSide) return users;
        const q = searchValue.toLowerCase();
        if (!q) return users;
        return searchOption === searchOptions[0]
            ? users.filter((u) => u.email?.toLowerCase().includes(q))
            : users.filter((u) => u.id.toLowerCase().includes(q));
    });

    let session = useSession('admin');
    // deleting users stays full-admin only (like in the details); nobody deletes their own account here
    const remove: RowDeletion<UserResponseSimple> = {
        label: x => x.email,
        canRemove: x => session.isAdmin() && x.id !== session.get()?.user_id,
        run: async x => {
            let res = await fetchDelete(`/auth/v1/users/${x.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: () => fetchUsers(sspHeaders ? `page_size=${sspPageSize}` : undefined),
    };

    onMount(() => {
        fetchUsers('page_size=' + sspPageSize);
        fetchUserValuesConfig();
        fetchAuthProviders();
        fetchRoles();
        fetchGroups();
    });

    $effect(() => {
        const q = searchValue.toLowerCase();
        if (useServerSide) {
            if (q.length < 3) {
                if (isSearchedServer) {
                    fetchUsers('page_size=' + sspPageSize);
                    isSearchedServer = false;
                }
            } else {
                searchServer(q);
            }
        }
    });

    async function searchServer(q: string) {
        isSearchedServer = true;
        const idx: SearchParamsIdxUser = searchOption === searchOptions[0] ? 'email' : 'id';
        let res = await fetchSearchServer<UserResponseSimple[]>({ ty: 'user', idx, q });
        if (res.body) {
            users = res.body;
        } else {
            console.error(res.error);
        }
    }

    async function fetchUserValuesConfig() {
        let res = await fetchGet<UserValuesConfig>('/auth/v1/users/values_config');
        if (res.body) {
            userValuesConfig = res.body;
        } else {
            console.error(res);
        }
    }

    async function fetchUsers(urlParams?: string): Promise<[number, Headers]> {
        fetching = true;
        let url = `/auth/v1/users`;
        if (urlParams) {
            url += `?${urlParams}`;
        }

        let res = await fetchGet<UserResponse[]>(url);
        if (res.error) {
            err = 'Error fetching users: ' + res.error.message;
        } else if (res.body) {
            err = '';
            if (res.status === 206) {
                useServerSide = true;
                parseSspHeaders(res.headers);
            } else {
                useServerSide = false;
                sspHeaders = undefined;
            }
            users = res.body;
        }
        fetching = false;

        return [res.status, res.headers];
    }

    function parseSspHeaders(headers: Headers) {
        sspHeaders = headers;
        sspPageSize = Number.parseInt(headers.get('x-page-size') || '20');
        sspToken = headers.get('x-continuation-token');
        sspTotal = Number.parseInt(headers.get('x-user-count') || '0');
    }

    // The server only knows "next" and "previous" pages (continuation tokens), so jumps walk there.
    async function handleFetch(pg: PaginationState) {
        if (pg.pageSize !== sspPageSize) {
            sspPageSize = pg.pageSize;
            sspToken = undefined;
            await fetchUsers(`page_size=${pg.pageSize}`);
            sspPage = 1;
            return;
        }
        const target = pg.pageIndex + 1;
        if (target === 1) {
            sspToken = undefined;
            await fetchUsers(`page_size=${sspPageSize}`);
            sspPage = 1;
            return;
        }
        while (sspPage !== target) {
            const backwards = target < sspPage;
            let params = `page_size=${sspPageSize}`;
            if (backwards) {
                if (sspPage === 2) {
                    sspToken = undefined;
                    await fetchUsers(params);
                    sspPage = 1;
                    continue;
                }
                params += `&backwards=true&offset=${users.length - 1}`;
            }
            if (sspToken) {
                params += `&continuation_token=${sspToken}`;
            }
            await fetchUsers(params);
            sspPage += backwards ? -1 : 1;
        }
    }

    async function fetchAuthProviders() {
        let res = await fetchGet<AuthProviderTemplate[]>('/auth/v1/providers/minimal');
        if (res.body) {
            providers = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchRoles() {
        let res = await fetchGet<RoleResponse[]>('/auth/v1/roles');
        if (res.body) {
            roles = res.body.toSorted((a, b) => a.name.localeCompare(b.name));
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchGroups() {
        let res = await fetchGet<GroupResponse[]>('/auth/v1/groups');
        if (res.body) {
            groups = res.body.toSorted((a, b) => a.name.localeCompare(b.name));
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function onAddNew(id: string, another?: boolean) {
        if (another) {
            addNonce++;
            toast.success('Saved');
            await fetchUsers();
            return;
        }
        closeModal?.();
        await fetchUsers();
        uid.set(id);
    }

    // the deleted user's panel must not stay open
    function onDeleted() {
        uid.set(undefined);
    }

    function onSave() {
        fetchUsers();
        fetchRoles();
        fetchGroups();
        searchValue = '';
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="users" class="min-h-0">
        <DataTable
            data={usersShown}
            {columns}
            clientSide={!sspHeaders}
            paginationData={sspHeaders
                ? { page: sspPage, limit: sspPageSize, totalRows: sspTotal }
                : undefined}
            {handleFetch}
            {fetching}
            onFetch={() => fetchUsers(sspHeaders ? `page_size=${sspPageSize}` : undefined)}
            getRowId={(u) => u.id}
            remove={session.isAdmin() ? remove : undefined}
            onRowClick={(u) => {
                uid.set(u.id);
                tr.trigger('navSubSub');
            }}
            initialSorting={[{ id: 'created_at', desc: true }]}
        >
            {#snippet toolbar()}
                <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
                    <div class="relative w-full min-w-48 sm:w-72">
                        <Search class="text-muted-foreground absolute top-1/2 left-2.5 size-4 -translate-y-1/2" />
                        <Input class="h-8 pl-8" placeholder={t.common.search} aria-label={t.common.search} bind:value={searchValue} />
                    </div>
                    <div class="w-28">
                        <Options ariaLabel={t.common.search} options={searchOptions} bind:value={searchOption} />
                    </div>
                </div>
                <div class="flex items-center gap-1">
                    <SendMail {groups} {roles} />
                    <EmailJobs />
                    <ButtonAddModal bind:ref={refAddNew} level={roles.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                            <UserAddNew config={userValuesConfig} onSave={onAddNew} {roles} {groups} />
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!(userId && userValuesConfig)} title={users.find((u) => u.id === userId)?.email ?? ta.nav.users} onClose={() => uid.set(undefined)} wide>
    {#if userId && userValuesConfig}
        {#key userId}
            <UserDetails {userValuesConfig} {userId} {providers} {roles} {groups} {onSave} {onDeleted} />
        {/key}
    {/if}
</DetailsSheet>
