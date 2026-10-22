<script lang="ts">
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import Tabs from '$lib/tabs/Tabs.svelte';
    import { fetchGet } from '$api/fetch';
    import type { PamGroupResponse, PamHostSimpleResponse, PamUserResponse } from '$api/types/pam';
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


    const userColumns: FeaturedColumnDef<PamUserResponse>[] = [
        textCol<PamUserResponse>('name', ta.pam.username, { size: 220 }),
        textCol<PamUserResponse>('email', 'E-Mail', { size: 280 }),
        textCol<PamUserResponse>('id', 'UID', { size: 100 }),
        textCol<PamUserResponse>('gid', 'GID', { size: 100 }),
        textCol<PamUserResponse>('shell', 'Shell', { size: 180 }),
    ];
    const groupColumns: FeaturedColumnDef<PamGroupResponse>[] = [
        textCol<PamGroupResponse>('name', ta.pam.groupName, { size: 260 }),
        textCol<PamGroupResponse>('typ', ta.pam.groupType, { size: 140 }),
        textCol<PamGroupResponse>('id', 'GID', { size: 100 }),
    ];
    const hostColumns: FeaturedColumnDef<PamHostSimpleResponse>[] = [
        textCol<PamHostSimpleResponse>('name', 'Hostname', { size: 260 }),
        fnTextCol<PamHostSimpleResponse>('aliases', 'Aliases', h => (h.aliases ?? []).join(', '), { size: 260 }),
        fnTextCol<PamHostSimpleResponse>('addresses', 'IPs', h => (h.addresses ?? []).join(', '), { size: 260 }),
    ];

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
        if (res.body) {
            hosts = res.body;
        } else {
            err = res.error?.message || 'ERROR';
        }
    }

    async function fetchUsers() {
        let res = await fetchGet<PamUserResponse[]>('/auth/v1/pam/users');
        if (res.body) {
            users = res.body;
        } else {
            err = res.error?.message || 'ERROR';
        }
    }

    function onCreateGroup(group: PamGroupResponse) {
        closeModal?.();
        fetchGroups();
        selectedGroup = group;
    }

    function onCreateHost(host: PamHostSimpleResponse) {
        closeModal?.();
        fetchHosts();
        selectedHost = host;
    }

    function onCreateUser(user: PamUserResponse) {
        closeModal?.();
        fetchUsers();
        fetchGroups();
        selectedUser = user;
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
                    {#if isUser}
                        <PAMUserAdd onCreate={onCreateUser} />
                    {:else if isGroup}
                        <PAMGroupAdd {groups} onCreate={onCreateGroup} />
                    {:else if isHost}
                        <PamHostAdd {groups} onCreate={onCreateHost} />
                    {/if}
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
                onRowClick={g => (selectedGroup = g)}
                onFetch={fetchGroups}
                toolbar={addButton}
                emptyText={ta.common.noEntries}
            />
        {:else if isHost}
            <DataTable
                data={hosts}
                columns={hostColumns}
                clientSide
                searchable
                getRowId={h => h.id}
                onRowClick={h => (selectedHost = h)}
                onFetch={fetchHosts}
                toolbar={addButton}
                emptyText={ta.common.noEntries}
            />
        {/if}
    </div>
</ContentAdmin>

<DetailsSheet
    open={(isUser && !!selectedUser) || (isGroup && !!selectedGroup) || (isHost && !!selectedHost)}
    title={isUser ? (selectedUser?.name ?? '') : isGroup ? (selectedGroup?.name ?? '') : (selectedHost?.name ?? '')}
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
    {:else if isGroup && selectedGroup}
        {#key selectedGroup.id}
            <PAMGroupDetails
                group={selectedGroup}
                onDelete={() => {
                    fetchGroups();
                    selectedGroup = undefined;
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
