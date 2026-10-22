<script lang="ts">
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchGet } from '$api/fetch';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { RoleResponse } from '$api/types/roles.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import RoleAddNew from '$lib5/admin/roles/RoleAddNew.svelte';
    import RoleDetails from '$lib5/admin/roles/RoleDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let roles: RoleResponse[] = $state([]);
    let role: undefined | RoleResponse = $derived(roles.find(x => x.id === rid.get()));
    let rid = useParam('rid');

    const columns: FeaturedColumnDef<RoleResponse>[] = [
        textCol<RoleResponse>('name', ta.common.name, { size: 260 }),
        textCol<RoleResponse>('id', 'ID', { size: 260 }),
    ];

    onMount(() => {
        fetchRoles();
    });

    async function fetchRoles() {
        let res = await fetchGet<RoleResponse[]>('/auth/v1/roles');
        if (res.body) {
            roles = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchRoles();
    }

    async function onAddNew(id: string) {
        closeModal?.();
        await fetchRoles();
        rid.set(id);
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="roles" class="min-h-0">
        <DataTable
            data={roles}
            {columns}
            clientSide
            searchable
            getRowId={x => x.id}
            onRowClick={x => rid.set(x.id)}
            onFetch={fetchRoles}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={roles.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        <RoleAddNew onSave={onAddNew} {roles} />
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!role} title={role?.name ?? ''} onClose={() => rid.set(undefined)}>
    {#if role}
        {#key role.id}
            <RoleDetails {role} {roles} {onSave} />
        {/key}
    {/if}
</DetailsSheet>
