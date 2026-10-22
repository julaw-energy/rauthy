<script lang="ts">
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchGet } from '$api/fetch';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { UserAttrConfigResponse, UserAttrConfigValueResponse } from '$api/types/user_attrs.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { ATTR_TYPES } from '$lib5/attrTypes';
    import { boolCol, fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import AttrAddNew from '$lib5/admin/attrs/AttrAddNew.svelte';
    import AttrDetails from '$lib5/admin/attrs/AttrDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let attrs: UserAttrConfigValueResponse[] = $state([]);
    let an = useParam('an');
    let attr: undefined | UserAttrConfigValueResponse = $derived(attrs.find(a => a.name === an.get()));

    const columns: FeaturedColumnDef<UserAttrConfigValueResponse>[] = [
        textCol<UserAttrConfigValueResponse>('name', ta.common.name, { size: 240 }),
        textCol<UserAttrConfigValueResponse>('desc', ta.attrs.desc, { size: 320 }),
        fnTextCol<UserAttrConfigValueResponse>('default_value', ta.attrs.defaultValue, a =>
            a.default_value === undefined || a.default_value === null ? '' : JSON.stringify(a.default_value),
        ),
        fnTextCol<UserAttrConfigValueResponse>(
            'typ',
            'Type',
            a => ATTR_TYPES.find(t => t.value === a.typ)?.label ?? '',
        ),
        boolCol<UserAttrConfigValueResponse>('user_editable', ta.attrs.userEditable),
    ];

    onMount(() => {
        fetchAttrs();
    });

    async function fetchAttrs() {
        let res = await fetchGet<UserAttrConfigResponse>('/auth/v1/users/attr');
        if (res.body) {
            attrs = res.body.values;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchAttrs();
    }

    async function onAddNew(name: string) {
        closeModal?.();
        await fetchAttrs();
        an.set(name);
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="attrs" class="min-h-0">
        <DataTable
            data={attrs}
            {columns}
            clientSide
            searchable
            getRowId={a => a.name}
            onRowClick={a => an.set(a.name)}
            onFetch={fetchAttrs}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={attrs.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        <AttrAddNew onSave={onAddNew} {attrs} />
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!attr} title={attr?.name ?? ''} onClose={() => an.set(undefined)}>
    {#if attr}
        {#key attr.name}
            <AttrDetails {attr} {attrs} {onSave} />
        {/key}
    {/if}
</DetailsSheet>
