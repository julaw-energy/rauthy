<script lang="ts">
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchGet } from '$api/fetch';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { ApiKeyResponse, ApiKeysResponse } from '$api/types/api_keys.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { dateCol, fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import ApiKeyAddNew from '$lib5/admin/api_keys/ApiKeyAddNew.svelte';
    import ApiKeyDetails from '$lib5/admin/api_keys/ApiKeyDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let keys: ApiKeyResponse[] = $state([]);
    let kid = useParam('kn');
    let key: undefined | ApiKeyResponse = $derived(keys.find(k => k.name === kid.get()));

    const columns: FeaturedColumnDef<ApiKeyResponse>[] = [
        textCol<ApiKeyResponse>('name', ta.api_key.keyName, { size: 260 }),
        dateCol<ApiKeyResponse>('created', 'Created'),
        dateCol<ApiKeyResponse>('expires', ta.api_key.expires),
        fnTextCol<ApiKeyResponse>('access', 'Access', k => (k.access ?? []).map(a => a.group).join(', '), { size: 360 }),
    ];

    onMount(() => {
        fetchApiKeys();
    });

    async function fetchApiKeys() {
        let res = await fetchGet<ApiKeysResponse>('/auth/v1/api_keys');
        if (res.body) {
            keys = res.body.keys;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onNewKey() {
        closeModal?.();
        fetchApiKeys();
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="keys" class="min-h-0" aria-label={ta.common.details}>
        <DataTable
            data={keys}
            {columns}
            clientSide
            searchable
            getRowId={k => k.name}
            onRowClick={k => kid.set(k.name)}
            onFetch={fetchApiKeys}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={keys.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        <ApiKeyAddNew {keys} onSave={onNewKey} />
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!key} title={key?.name ?? ''} onClose={() => kid.set(undefined)} wide>
    {#if key}
        {#key key.name}
            <ApiKeyDetails bind:key onSave={fetchApiKeys} />
        {/key}
    {/if}
</DetailsSheet>
