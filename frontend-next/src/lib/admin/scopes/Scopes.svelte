<script lang="ts">
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchGet } from '$api/fetch';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { ScopeResponse } from '$api/types/scopes.ts';
    import type { UserAttrConfigResponse, UserAttrConfigValueResponse } from '$api/types/user_attrs.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { boolCol, fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import ScopeAddNew from '$lib5/admin/scopes/ScopeAddNew.svelte';
    import ScopeDetails from '$lib5/admin/scopes/ScopeDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let sid = useParam('sid');
    let attrs: UserAttrConfigValueResponse[] = $state([]);
    let scopes: ScopeResponse[] = $state([]);
    let scope: undefined | ScopeResponse = $derived(scopes.find(s => s.id === sid.get()));

    const columns: FeaturedColumnDef<ScopeResponse>[] = [
        textCol<ScopeResponse>('name', ta.common.name, { size: 220 }),
        textCol<ScopeResponse>('id', 'ID', { size: 240 }),
        fnTextCol<ScopeResponse>('attr_include_access', 'Access Token', s => (s.attr_include_access ?? []).join(', '), { size: 240 }),
        fnTextCol<ScopeResponse>('attr_include_id', 'ID Token', s => (s.attr_include_id ?? []).join(', '), { size: 240 }),
        boolCol<ScopeResponse>('claims_at_root', ta.scopes.claimsAtRoot, { size: 150 }),
    ];

    onMount(() => {
        fetchScopes();
        fetchAttrs();
    });

    async function fetchScopes() {
        let res = await fetchGet<ScopeResponse[]>('/auth/v1/scopes');
        if (res.body) {
            scopes = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchAttrs() {
        let res = await fetchGet<UserAttrConfigResponse>('/auth/v1/users/attr');
        if (res.body) {
            attrs = res.body.values;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchScopes();
    }

    async function onAddNew(id: string) {
        closeModal?.();
        await fetchScopes();
        sid.set(id);
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="scopes" class="min-h-0">
        <DataTable
            data={scopes}
            {columns}
            clientSide
            searchable
            getRowId={s => s.id}
            onRowClick={s => sid.set(s.id)}
            onFetch={fetchScopes}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={scopes.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        <ScopeAddNew onSave={onAddNew} {scopes} />
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!scope} title={scope?.name ?? ''} onClose={() => sid.set(undefined)}>
    {#if scope}
        {#key scope.id}
            <ScopeDetails {attrs} {scope} {scopes} {onSave} />
        {/key}
    {/if}
</DetailsSheet>
