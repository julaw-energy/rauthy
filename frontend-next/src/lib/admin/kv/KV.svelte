<script lang="ts">
    import { onMount } from 'svelte';
    import { fetchGet, fetchPost } from '$api/fetch';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import DetailsSheet from '$lib5/DetailsSheet.svelte';
    import { useParam } from '$state/param.svelte';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import type { KVNamespaceRequest, KVNamespaceResponse } from '$api/types/kv';
    import KVDetails from '$lib/admin/kv/KVDetails.svelte';
    import Tabs from '$lib/tabs/Tabs.svelte';
    import KVAccess from '$lib/admin/kv/KVAccess.svelte';
    import KVDelete from '$lib/admin/kv/KVDelete.svelte';
    import Form from '$lib/form/Form.svelte';
    import Input from '$lib/form/Input.svelte';
    import AddFooter from '$lib/form/AddFooter.svelte';
    import { toast } from 'svelte-sonner';
    import { useI18n } from '$state/i18n.svelte';
    import { PATTERN_GROUP } from '$utils/patterns';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import KVNamespaceEdit from '$lib/admin/kv/KVNamespaceEdit.svelte';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { boolCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';

    let t = useI18n();
    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let refInput: undefined | HTMLInputElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());
    $effect(() => {
        refInput?.focus();
    });

    let closeModal: undefined | (() => void) = $state();

    let ns = useParam('ns');
    let err = $state('');
    let tab = $state(ta.kv.tabs[0]);

    let namespaces: KVNamespaceResponse[] = $state([]);
    let selected: undefined | KVNamespaceResponse = $derived(
        namespaces.find(namespace => namespace.name === ns.get()),
    );

    const columns: FeaturedColumnDef<KVNamespaceResponse>[] = [
        textCol<KVNamespaceResponse>('name', ta.common.name, { size: 320 }),
        boolCol<KVNamespaceResponse>('public', 'Public Access', { size: 150 }),
    ];

    onMount(() => {
        fetchData();
    });

    async function fetchData() {
        let res = await fetchGet<KVNamespaceResponse[]>('/auth/v1/kv/ns');
        if (res.body) {
            namespaces = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onDelete() {
        ns.set(undefined);
        closeModal?.();
        fetchData();
        tab = ta.kv.tabs[0];
    }

    async function onSave(nameNew: string) {
        await fetchData();
        ns.set(nameNew);
    }

    // bumped by "save & add another" to remount the add form empty
    let addNonce = $state(0);
    let another = $state(false);

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        let payload: KVNamespaceRequest = {
            name: params.get('name') || '',
            public: params.get('public') === 'on',
        };
        let res = await fetchPost(form.action, payload);
        if (res.error) {
            err = res.error.message;
            closeModal?.();
            return;
        }
        await fetchData();
        if (another) {
            addNonce++;
            toast.success('Saved');
        } else {
            closeModal?.();
        }
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="kv" class="min-h-0" aria-label={ta.common.details}>
        <DataTable
            data={namespaces}
            {columns}
            clientSide
            searchable
            getRowId={n => n.name}
            onRowClick={n => ns.set(n.name)}
            onFetch={fetchData}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={namespaces.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                        <Form action="/auth/v1/kv/ns" {onSubmit}>
                            <h3 class="text-lg font-semibold">{ta.kv.addNewNs}</h3>
                            <Input
                                bind:ref={refInput}
                                name="name"
                                label={ta.common.name}
                                placeholder={ta.common.name}
                                pattern={PATTERN_GROUP}
                            />
                            <InputCheckbox ariaLabel="Public Access" name="public" checked={false}>
                                Public Access
                            </InputCheckbox>

                            <AddFooter bind:another />
                        </Form>
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>

<DetailsSheet open={!!selected} title={selected?.name ?? ''} onClose={() => ns.set(undefined)} wide>
    {#if selected}
        {#key selected.name}
            <div class="grid gap-4">
                <Tabs tabs={ta.kv.tabs} bind:selected={tab} />

                {#if tab === ta.kv.tabs[0]}
                    <KVDetails ns={selected} />
                {:else if tab === ta.kv.tabs[1]}
                    <KVAccess {ns} />
                {:else if tab === ta.kv.tabs[2]}
                    <KVNamespaceEdit ns={selected} {onSave} />
                {:else if tab === ta.kv.tabs[3]}
                    <KVDelete {ns} {onDelete} />
                {/if}
            </div>
        {/key}
    {/if}
</DetailsSheet>
