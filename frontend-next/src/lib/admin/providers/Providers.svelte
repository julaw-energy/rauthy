<script lang="ts">
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchPost } from '$api/fetch';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { ProviderResponse } from '$api/types/auth_provider.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { boolCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import ProviderAddNew from '$lib5/admin/providers/ProviderAddNew.svelte';
    import ProviderDetails from '$lib5/admin/providers/ProviderDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    let closeModal: undefined | (() => void) = $state();

    let pid = useParam('pid');
    let err = $state('');
    let providers: ProviderResponse[] = $state([]);
    let provider: undefined | ProviderResponse = $derived(providers.find(p => p.id === pid.get()));

    const columns: FeaturedColumnDef<ProviderResponse>[] = [
        textCol<ProviderResponse>('name', ta.common.name, { size: 220 }),
        textCol<ProviderResponse>('typ', 'Type', { size: 140 }),
        textCol<ProviderResponse>('issuer', 'Issuer', { size: 320 }),
        boolCol<ProviderResponse>('enabled', ta.common.enabled),
        boolCol<ProviderResponse>('auto_onboarding', 'Auto-Onboarding', { size: 160 }),
        boolCol<ProviderResponse>('auto_link', ta.providers.config.autoLink, { size: 140 }),
        boolCol<ProviderResponse>('use_pkce', 'PKCE', { size: 90 }),
    ];

    onMount(() => {
        fetchData();
    });

    async function fetchData() {
        let res = await fetchPost<ProviderResponse[]>('/auth/v1/providers');
        if (res.body) {
            providers = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        pid.set(undefined);
        closeModal?.();
        fetchData();
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="federation" class="min-h-0" aria-label={ta.common.details}>
        <DataTable
            data={providers}
            {columns}
            clientSide
            searchable
            getRowId={p => p.id}
            onRowClick={p => pid.set(p.id)}
            onFetch={fetchData}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={providers.length === 0 ? 1 : 2} bind:closeModal>
                        <ProviderAddNew {onSave} />
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!provider} title={provider?.name ?? ''} onClose={() => pid.set(undefined)} wide>
    {#if provider}
        {#key provider.id}
            <ProviderDetails bind:provider onSave={fetchData} />
        {/key}
    {/if}
</DetailsSheet>
