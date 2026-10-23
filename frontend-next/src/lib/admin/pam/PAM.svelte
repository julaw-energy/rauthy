<script lang="ts">
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import Tabs from '$lib/tabs/Tabs.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import { toast } from 'svelte-sonner';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import type { EditConfig } from '$lib/components/data-table/data-table.svelte';
    import { PATTERN_LINUX_HOSTNAME } from '$utils/patterns';
    import type {
        PamGroupResponse,
        PamHostDetailsResponse,
        PamHostSimpleResponse,
        PamUserDetailsResponse,
        PamUserResponse,
    } from '$api/types/pam';
    import { onMount } from 'svelte';
    import PAMGroupDetails from '$lib/admin/pam/groups/PAMGroupDetails.svelte';
    import PamHostAdd from '$lib/admin/pam/hosts/PAMHostAdd.svelte';
    import PAMHostDetails from '$lib/admin/pam/hosts/PAMHostDetails.svelte';
    import PAMGroupAdd from '$lib/admin/pam/groups/PAMGroupAdd.svelte';
    import PAMUserAdd from '$lib/admin/pam/users/PAMUserAdd.svelte';
    import UserPicture from '$lib/UserPicture.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import PAMUserDetails from '$lib/admin/pam/users/PAMUserDetails.svelte';
    import type { PamGroupsSorted } from '$lib/admin/pam/types';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);
    let closeModal: undefined | (() => void) = $state();
    let searchValue = $state('');

    const tabs = [ta.nav.users, ta.pam.groups, 'Hosts'];
    let selectedNav = $state(tabs[0]);

    let selectedUser: undefined | PamUserResponse = $state();
    let selectedGroup: undefined | PamGroupResponse = $state();
    let selectedHost: undefined | PamHostSimpleResponse = $state();

    let err = $state('');

    let groups: PamGroupResponse[] = $state([]);
    let groupsFiltered: PamGroupResponse[] = $derived.by(() => {
        let s = searchValue.toLowerCase();
        return groups.filter(g => g.name.toLowerCase().includes(s));
    });
    let groupsSorted: PamGroupsSorted = $derived.by(() => {
        let generic: PamGroupResponse[] = [];
        let host: PamGroupResponse[] = [];
        let local: PamGroupResponse[] = [];
        let user: PamGroupResponse[] = [];

        for (let group of groups) {
            switch (group.typ) {
                case 'generic':
                    generic.push(group);
                    break;
                case 'host':
                    host.push(group);
                    break;
                case 'local':
                    local.push(group);
                    break;
                case 'user':
                    user.push(group);
                    break;
            }
        }

        generic.sort((a, b) => a.name.localeCompare(b.name));
        host.sort((a, b) => a.name.localeCompare(b.name));
        local.sort((a, b) => a.name.localeCompare(b.name));
        user.sort((a, b) => a.name.localeCompare(b.name));

        return { generic, host, local, user };
    });
    let hosts: PamHostSimpleResponse[] = $state([]);
    let hostsFiltered: PamHostSimpleResponse[] = $derived.by(() => {
        let s = searchValue.toLowerCase();
        return hosts.filter(h => h.name.toLowerCase().includes(s));
    });
    let users: PamUserResponse[] = $state([]);
    let usersFiltered: PamUserResponse[] = $derived.by(() => {
        let s = searchValue.toLowerCase();
        return users.filter(u => u.name.toLowerCase().includes(s));
    });


    // ---- hosts: the list endpoint only has names / addresses, the rest comes from the details --------
    interface HostRow {
        id: string;
        name: string;
        gid: number;
        aliases: string[];
        ips: string[];
        force_mfa: boolean;
        local_password_only: boolean;
        notes: string;
    }
    let hostRows: HostRow[] = $state([]);

    const joinList = (l?: string[]) => (l ?? []).join(', ');
    const splitList = (t: string) =>
        t
            .split(',')
            .map(x => x.trim())
            .filter(Boolean);
    const hostGroupItems = $derived(groups.filter(g => g.typ === 'host').map(g => ({ value: String(g.id), label: g.name })));

    const userColumns: FeaturedColumnDef<PamUserResponse>[] = [
        textCol<PamUserResponse>('id', 'UID', { size: 100 }),
        textCol<PamUserResponse>('name', ta.pam.username, { size: 200 }),
        textCol<PamUserResponse>('email', 'E-Mail', { size: 260 }),
        textCol<PamUserResponse>('gid', 'GID', { size: 100 }),
        textCol<PamUserResponse>('shell', 'Shell', { size: 180 }),
        fnTextCol<PamUserResponse>('home_dir', 'Home', u => u.home_dir ?? '', { size: 220 }),
    ];
    const groupColumns: FeaturedColumnDef<PamGroupResponse>[] = [
        textCol<PamGroupResponse>('id', 'GID', { size: 100 }),
        textCol<PamGroupResponse>('name', ta.pam.groupName, { size: 260 }),
        textCol<PamGroupResponse>('typ', ta.pam.groupType, { size: 140 }),
    ];
    const hostColumns: FeaturedColumnDef<HostRow>[] = [
        textCol<HostRow>('id', 'ID', { size: 220 }),
        textCol<HostRow>('name', 'Hostname', { size: 200 }),
        fnTextCol<HostRow>('gid', 'Group', h => hostGroupItems.find(g => g.value === String(h.gid))?.label ?? String(h.gid), { size: 180 }),
        fnTextCol<HostRow>('aliases', 'Aliases', h => joinList(h.aliases), { size: 220 }),
        fnTextCol<HostRow>('ips', 'IPs', h => joinList(h.ips), { size: 220 }),
        fnTextCol<HostRow>('force_mfa', 'MFA', h => (h.force_mfa ? '✓' : '–'), { size: 90 }),
        fnTextCol<HostRow>('local_password_only', 'Local pw only', h => (h.local_password_only ? '✓' : '–'), { size: 140 }),
        textCol<HostRow>('notes', 'Notes', { size: 240 }),
    ];

    const userEdit: EditConfig<PamUserResponse> = {
        columns: {
            shell: { required: true, maxLength: 24, read: u => u.shell },
            home_dir: { maxLength: 128, read: u => u.home_dir ?? '' },
        },
        save: async (u, ch) => {
            // the update endpoint also wants the group memberships, which the list does not carry
            let det = await fetchGet<PamUserDetailsResponse>(`/auth/v1/pam/users/${u.id}`);
            if (!det.body) throw new Error(det.error?.message || 'Error');
            let res = await fetchPut(`/auth/v1/pam/users/${u.id}`, {
                shell: ch.shell ?? u.shell,
                home_dir: (ch.home_dir ?? u.home_dir) || undefined,
                groups: det.body.groups,
            });
            if (res.error) throw new Error(res.error.message);
        },
        onSaved: fetchUsers,
    };

    const hostEdit: EditConfig<HostRow> = {
        columns: {
            name: { required: true, maxLength: 63, pattern: PATTERN_LINUX_HOSTNAME },
            gid: {
                kind: 'select',
                // a getter: the groups are fetched after this object is created
                get items() {
                    return hostGroupItems;
                },
                read: h => String(h.gid),
            },
            aliases: { read: h => joinList(h.aliases), placeholder: 'alias1, alias2' },
            ips: { read: h => joinList(h.ips), placeholder: '10.0.0.1, fd00::1' },
            force_mfa: { kind: 'bool', read: h => String(h.force_mfa) },
            local_password_only: { kind: 'bool', read: h => String(h.local_password_only) },
            notes: { read: h => h.notes },
        },
        save: async (h, ch) => {
            let res = await fetchPut(`/auth/v1/pam/hosts/${h.id}`, {
                hostname: ch.name ?? h.name,
                gid: Number(ch.gid ?? h.gid),
                force_mfa: (ch.force_mfa ?? String(h.force_mfa)) === 'true',
                local_password_only: (ch.local_password_only ?? String(h.local_password_only)) === 'true',
                notes: (ch.notes ?? h.notes).trim() || undefined,
                ips: splitList(ch.ips ?? joinList(h.ips)),
                aliases: splitList(ch.aliases ?? joinList(h.aliases)),
            });
            if (res.status === 422) throw new Error('Invalid IP address');
            if (res.error) throw new Error(res.error.message);
        },
        onSaved: fetchHosts,
    };

    const userRemove: RowDeletion<PamUserResponse> = {
        label: u => u.name,
        run: async u => {
            let res = await fetchDelete(`/auth/v1/pam/users/${u.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: async () => {
            await fetchUsers();
            await fetchGroups();
        },
    };
    const groupRemove: RowDeletion<PamGroupResponse> = {
        label: g => g.name,
        // `immutable` and `user` groups are managed by Rauthy; host groups only while no host uses them
        canRemove: g => g.typ === 'generic' || g.typ === 'local' || g.typ === 'host',
        run: async g => {
            let res = await fetchDelete(`/auth/v1/pam/groups/${g.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchGroups,
    };
    const hostRemove: RowDeletion<HostRow> = {
        label: h => h.name,
        run: async h => {
            let res = await fetchDelete(`/auth/v1/pam/hosts/${h.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchHosts,
    };

    let isUser = $derived(selectedNav === tabs[0]);
    let isGroup = $derived(selectedNav === tabs[1]);
    let isHost = $derived(selectedNav === tabs[2]);

    onMount(() => {
        fetchGroups();
        fetchHosts();
        fetchUsers();
    });

    $effect(() => {
        if (isUser || isGroup || isHost) {
            searchValue = '';
        }
    });

    async function fetchGroups() {
        let res = await fetchGet<PamGroupResponse[]>('/auth/v1/pam/groups');
        if (res.body) {
            groups = res.body;
        } else {
            err = res.error?.message || 'ERROR';
        }
    }

    async function fetchHosts() {
        let res = await fetchGet<PamHostSimpleResponse[]>('/auth/v1/pam/hosts');
        if (!res.body) {
            err = res.error?.message || 'ERROR';
            return;
        }
        hosts = res.body;
        // one request per host for the fields the list does not have (the secret is not kept)
        let rows = await Promise.all(
            res.body.map(async (h): Promise<HostRow | undefined> => {
                let d = await fetchGet<PamHostDetailsResponse>(`/auth/v1/pam/hosts/${h.id}`);
                if (!d.body) return undefined;
                return {
                    id: d.body.id,
                    name: d.body.hostname,
                    gid: d.body.gid,
                    aliases: d.body.aliases ?? [],
                    ips: d.body.ips ?? [],
                    force_mfa: d.body.force_mfa,
                    local_password_only: d.body.local_password_only,
                    notes: (d.body.notes ?? '').trim(),
                };
            }),
        );
        hostRows = rows.filter((r): r is HostRow => !!r);
    }

    async function fetchUsers() {
        let res = await fetchGet<PamUserResponse[]>('/auth/v1/pam/users');
        if (res.body) {
            users = res.body;
        } else {
            err = res.error?.message || 'ERROR';
        }
    }

    function afterAdd(another?: boolean): boolean {
        if (another) {
            addNonce++;
            toast.success('Saved');
            return true;
        }
        closeModal?.();
        return false;
    }

    function onCreateGroup(group: PamGroupResponse, another?: boolean) {
        afterAdd(another);
        fetchGroups();
    }

    function onCreateHost(host: PamHostSimpleResponse, another?: boolean) {
        afterAdd(another);
        fetchHosts();
    }

    function onCreateUser(user: PamUserResponse, another?: boolean) {
        afterAdd(another);
        fetchUsers();
        fetchGroups();
    }
</script>

<ContentAdmin>
    <div class="grid min-h-0 gap-3" id="pam">
        {#if err}
            <p class="text-destructive text-sm">{err}</p>
        {/if}

        <Tabs {tabs} bind:selected={selectedNav} />

        {#snippet addButton()}
            <div class="ml-auto">
                <ButtonAddModal bind:ref={refAddNew} level={2} bind:closeModal alignRight>
                    {#key addNonce}
                        {#if isUser}
                            <PAMUserAdd onCreate={onCreateUser} />
                        {:else if isGroup}
                            <PAMGroupAdd {groups} onCreate={onCreateGroup} />
                        {:else if isHost}
                            <PamHostAdd {groups} onCreate={onCreateHost} />
                        {/if}
                    {/key}
                </ButtonAddModal>
            </div>
        {/snippet}

        {#if isUser}
            <DataTable
                data={users}
                columns={userColumns}
                clientSide
                searchable
                getRowId={u => String(u.id)}
                editable={userEdit}
                remove={userRemove}
                onRowClick={u => (selectedUser = u)}
                onFetch={fetchUsers}
                toolbar={addButton}
                emptyText={ta.common.noEntries}
            />
        {:else if isGroup}
            <DataTable
                data={groups}
                columns={groupColumns}
                clientSide
                searchable
                getRowId={g => String(g.id)}
                remove={groupRemove}
                onFetch={fetchGroups}
                toolbar={addButton}
                emptyText={ta.common.noEntries}
            />
        {:else if isHost}
            <DataTable
                data={hostRows}
                columns={hostColumns}
                clientSide
                searchable
                getRowId={h => h.id}
                editable={hostEdit}
                remove={hostRemove}
                onRowClick={h => (selectedHost = hosts.find(x => x.id === h.id))}
                onFetch={fetchHosts}
                toolbar={addButton}
                emptyText={ta.common.noEntries}
            />
        {/if}
    </div>
</ContentAdmin>

<DetailsSheet
    open={(isUser && !!selectedUser) || (isHost && !!selectedHost)}
    title={isUser ? (selectedUser?.name ?? '') : (selectedHost?.name ?? '')}
    onClose={() => {
        selectedUser = undefined;
        selectedGroup = undefined;
        selectedHost = undefined;
    }}
    wide
>
    {#if isUser && selectedUser}
        {#key selectedUser.id}
            <PAMUserDetails
                bind:user={selectedUser}
                {groupsSorted}
                onDelete={() => {
                    fetchUsers();
                    selectedUser = undefined;
                }}
            />
        {/key}
    {:else if isHost && selectedHost}
        {#key selectedHost.id}
            <PAMHostDetails
                hostSimple={selectedHost}
                {groups}
                onDelete={() => {
                    fetchHosts();
                    selectedHost = undefined;
                }}
            />
        {/key}
    {/if}
</DetailsSheet>
