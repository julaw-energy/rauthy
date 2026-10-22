<script lang="ts">
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchGet } from '$api/fetch';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { GroupResponse } from '$api/types/groups.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import GroupAddNew from '$lib5/admin/groups/GroupAddNew.svelte';
    import GroupDetails from '$lib5/admin/groups/GroupDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let groups: GroupResponse[] = $state([]);
    let group: undefined | GroupResponse = $derived(groups.find(x => x.id === gid.get()));
    let gid = useParam('gid');

    const columns: FeaturedColumnDef<GroupResponse>[] = [
        textCol<GroupResponse>('name', ta.common.name, { size: 260 }),
        textCol<GroupResponse>('id', 'ID', { size: 260 }),
    ];

    onMount(() => {
        fetchGroups();
    });

    async function fetchGroups() {
        let res = await fetchGet<GroupResponse[]>('/auth/v1/groups');
        if (res.body) {
            groups = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchGroups();
    }

    async function onAddNew(id: string) {
        closeModal?.();
        await fetchGroups();
        gid.set(id);
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="groups" class="min-h-0">
        <DataTable
            data={groups}
            {columns}
            clientSide
            searchable
            getRowId={x => x.id}
            onRowClick={x => gid.set(x.id)}
            onFetch={fetchGroups}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={groups.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        <GroupAddNew onSave={onAddNew} {groups} />
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!group} title={group?.name ?? ''} onClose={() => gid.set(undefined)}>
    {#if group}
        {#key group.id}
            <GroupDetails {group} {groups} {onSave} />
        {/key}
    {/if}
</DetailsSheet>
