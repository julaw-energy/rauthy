<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useParam } from '$state/param.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { ClientResponse } from '$api/types/clients.ts';
    import type { ScopeResponse } from '$api/types/scopes.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { boolCol, fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import ClientAddNew from '$lib5/admin/clients/ClientAddNew.svelte';
    import ClientDetails from '$lib5/admin/clients/ClientDetails.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);
    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let clients: ClientResponse[] = $state([]);
    let cid = useParam('cid');
    let client: undefined | ClientResponse = $derived(clients.find(c => c.id === cid.get()));
    let scopesAll: string[] = $state([]);

    const columns: FeaturedColumnDef<ClientResponse>[] = [
        textCol<ClientResponse>('id', 'ID', { size: 240 }),
        textCol<ClientResponse>('name', ta.clients.name, { size: 240 }),
        boolCol<ClientResponse>('enabled', ta.common.enabled),
        boolCol<ClientResponse>('confidential', ta.clients.confidential, { size: 130 }),
        boolCol<ClientResponse>('force_mfa', 'MFA', { size: 90 }),
        fnTextCol<ClientResponse>('flows_enabled', 'Flows', c => (c.flows_enabled ?? []).join(', '), { size: 300 }),
        fnTextCol<ClientResponse>('redirect_uris', 'Redirect URIs', c => (c.redirect_uris ?? []).join(', '), { size: 360 }),
    ];

    const remove: RowDeletion<ClientResponse> = {
        label: x => `${x.name || x.id} (${x.id})`,
        canRemove: x => x.id !== 'rauthy',
        run: async x => {
            let res = await fetchDelete(`/auth/v1/clients/${x.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchClients,
    };

    onMount(() => {
        fetchClients();
        fetchScopes();
    });

    async function fetchClients() {
        let res = await fetchGet<ClientResponse[]>('/auth/v1/clients');
        if (res.body) {
            clients = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchScopes() {
        let res = await fetchGet<ScopeResponse[]>('/auth/v1/scopes');
        if (res.body) {
            scopesAll = res.body.map(s => s.name);
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchClients();
    }

    async function onAddNew(id: string, another?: boolean) {
        if (another) {
            addNonce++;
            toast.success('Saved');
            await fetchClients();
            return;
        }
        closeModal?.();
        await fetchClients();
        cid.set(id);
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="clients" class="min-h-0">
        <DataTable
            data={clients}
            {columns}
            clientSide
            searchable
            getRowId={c => c.id}
            {remove}
            onRowClick={c => {
                cid.set(c.id);
                tr.trigger('navSubSub');
            }}
            onFetch={fetchClients}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={clients.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                            <ClientAddNew onSave={onAddNew} {clients} />
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!client} title={client?.name || client?.id || ''} onClose={() => cid.set(undefined)} wide>
    {#if client}
        {#key client.id}
            <ClientDetails {client} {clients} {scopesAll} {onSave} />
        {/key}
    {/if}
</DetailsSheet>
