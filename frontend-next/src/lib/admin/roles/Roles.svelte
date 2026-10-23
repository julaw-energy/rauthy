<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import { useTrigger } from '$state/callback.svelte';
    import type { RoleResponse } from '$api/types/roles.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import type { EditConfig } from '$lib/components/data-table/data-table.svelte';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import { PATTERN_ROLE_SCOPE } from '$utils/patterns';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import RoleAddNew from '$lib5/admin/roles/RoleAddNew.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);

    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let roles: RoleResponse[] = $state([]);

    const metaText = (x: RoleResponse) => (x.meta === undefined || x.meta === null ? '' : JSON.stringify(x.meta));

    const columns: FeaturedColumnDef<RoleResponse>[] = [
        textCol<RoleResponse>('id', 'ID', { size: 260 }),
        textCol<RoleResponse>('name', ta.common.name, { size: 260 }),
        fnTextCol<RoleResponse>('meta', 'Metadata (JSON)', x => metaText(x), { size: 420 }),
    ];

    const editable: EditConfig<RoleResponse> = {
        columns: {
            meta: {
                read: metaText,
                placeholder: '{"key": "value"}',
                validate: v => {
                    try {
                        JSON.parse(v);
                        return null;
                    } catch {
                        return 'Not valid JSON';
                    }
                },
            },
            name: { required: true, maxLength: 32, pattern: PATTERN_ROLE_SCOPE } },
        canEdit: x => !(x.name === 'rauthy_admin'),
        save: async (x, changes) => {
            if (changes.name !== undefined && roles.some(o => o.id !== x.id && o.name === changes.name)) {
                throw new Error(ta.common.nameExistsAlready);
            }
            let res = await fetchPut(`/auth/v1/roles/${x.id}`, { role: changes.name ?? x.name, meta: metaFrom(changes.meta, x.meta) });
            if (res.error) throw new Error(res.error.message);
        },
        onSaved: fetchRoles,
    };

    const metaFrom = (text: string | undefined, current: RoleResponse['meta']) =>
        text === undefined ? current : text.trim() === '' ? undefined : JSON.parse(text);

    const remove: RowDeletion<RoleResponse> = {
        label: x => x.name,
        canRemove: x => !(x.name === 'rauthy_admin'),
        run: async x => {
            let res = await fetchDelete(`/auth/v1/roles/${x.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchRoles,
    };

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

    async function onAddNew(id: string, another?: boolean) {
        if (another) {
            addNonce++;
            toast.success('Saved');
            await fetchRoles();
            return;
        }
        closeModal?.();
        await fetchRoles();
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
            {editable}
            {remove}
            rowActionsWidth={40}
            onFetch={fetchRoles}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={roles.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                            <RoleAddNew onSave={onAddNew} {roles} />
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>
