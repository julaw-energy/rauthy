<script lang="ts">
    import DataTable from '$lib/components/data-table/data-table.svelte'
    ;import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features'
    ;import { Badge } from '$lib/components/ui/badge';
    import type { KVNamespaceResponse, KVValueRequest, KVValueResponse } from '$api/types/kv';
    import Button from '$lib/button/Button.svelte';
    import Input from '$lib/form/Input.svelte';
    import SearchBar from '$lib/search_bar/SearchBar.svelte';
    import { fetchDelete, fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import Modal from '$lib/Modal.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Form from '$lib/form/Form.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import InputArea from '$lib/form/InputArea.svelte';
    import { PATTERN_KV_KEY } from '$utils/patterns';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { slide } from 'svelte/transition';
    import { parseJsonValue, stringifyJsonValue } from '$utils/jsonValue';
    import IconKey from '$icons/IconKey.svelte';
    import IconEdit from '$icons/IconEdit.svelte';
    import IconTrash from '$icons/IconTrash.svelte';
    import IconLink from '$icons/editor/IconLink.svelte';

    let {
        ns,
    }: {
        ns: KVNamespaceResponse;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let refInput: undefined | HTMLInputElement = $state();
    let refArea: undefined | HTMLTextAreaElement = $state();

    let showModalAdd = $state(false);
    let closeModalAdd: undefined | (() => void) = $state();
    let showModalEdit = $state(false);
    let closeModalEdit: undefined | (() => void) = $state();
    let showModalDelete = $state(false);
    let closeModalDelete: undefined | (() => void) = $state();

    let values: KVValueResponse[] = $state([]);
    let valuesSearch: KVValueResponse[] = $state([]);
    let entrySelected: KVValueResponse | undefined = $state();
    let limit = $state(1000);
    let err = $state('');
    let search = $state('');

    let encryptNew = $state(false);

    let urlValues = $derived(`/auth/v1/kv/ns/${ns.name}/values`);

    $effect(() => {
        // make sure to reset on nav-switch
        if (ns.name) {
            values = [];
        }
    });

    $effect(() => {
        if (search.length > 1) {
            searchValues();
        } else {
            valuesSearch = [];
        }
    });

    $effect(() => {
        if (refInput) {
            requestAnimationFrame(() => {
                refInput?.focus();
            });
        }
    });

    $effect(() => {
        if (refArea) {
            requestAnimationFrame(() => {
                refArea?.focus();
            });
        }
    });

    function openPubLink(key: string) {
        let link = `${window.location.origin}/auth/v1/kv/pub/${ns.name}/${key}`;
        window.open(link, '_blank')?.focus();
    }

    function editEntry(entry: KVValueResponse) {
        entrySelected = entry;
        showModalEdit = true;
    }

    function deleteEntry(entry: KVValueResponse) {
        entrySelected = entry;
        showModalDelete = true;
    }

    async function fetchAll() {
        if (err) {
            err = '';
        }
        if (search) {
            search = '';
        }

        let res = await fetchGet<KVValueResponse[]>(`${urlValues}?limit=${limit}`);
        if (res.body) {
            res.body.sort((a, b) => a.key.localeCompare(b.key));
            values = res.body;
        } else {
            err = res.error?.message || 'Failed to fetch values';
        }
    }

    async function searchValues() {
        if (err) {
            err = '';
        }

        let res = await fetchGet<KVValueResponse[]>(`${urlValues}?limit=${limit}&search=${search}`);
        if (res.body) {
            res.body.sort((a, b) => a.key.localeCompare(b.key));
            valuesSearch = res.body;
        } else {
            err = res.error?.message || 'Failed to fetch values';
        }
    }

    async function onSubmitAdd(form: HTMLFormElement, params: URLSearchParams) {
        let value = params.get('value');
        if (!value) {
            return;
        }

        let payload: KVValueRequest = {
            key: params.get('key') || '',
            encrypted: !!params.get('encrypted'),
            value: parseJsonValue(value),
        };

        let res = await fetchPost(form.action, payload);
        if (res.error) {
            err = res.error.message || 'Failed to add key';
        } else {
            closeModalAdd?.();
            if (values.length > 0) {
                await fetchAll();
            } else {
                search = payload.key;
            }
        }
    }

    async function onSubmitEdit(form: HTMLFormElement, params: URLSearchParams) {
        if (!entrySelected) {
            return;
        }
        let value = params.get('value')?.trim();
        if (!value) {
            return;
        }

        let payload: KVValueRequest = {
            key: entrySelected.key,
            encrypted: params.get('encrypted') === 'on' ? true : undefined,
            value: parseJsonValue(value),
        };

        let res = await fetchPut(form.action, payload);
        if (res.error) {
            err = res.error.message || 'Failed to update key';
        } else {
            closeModalEdit?.();

            if (valuesSearch.length > 0) {
                valuesSearch = valuesSearch.map(v => {
                    if (v.key === entrySelected?.key) {
                        v.value = value;
                        v.encrypted = payload.encrypted;
                    }
                    return v;
                });
            }
            if (values.length > 0) {
                values = values.map(v => {
                    if (v.key === entrySelected?.key) {
                        v.value = value;
                        v.encrypted = payload.encrypted;
                    }
                    return v;
                });
            }
        }
    }

    async function onSubmitDelete() {
        if (!entrySelected) {
            return;
        }

        let res = await fetchDelete(`${urlValues}/${entrySelected?.key}`);
        if (res.error) {
            err = res.error.message;
        } else {
            closeModalDelete?.();

            if (valuesSearch.length > 0) {
                valuesSearch = valuesSearch.filter(v => v.key !== entrySelected?.key);
            }
            if (values.length > 0) {
                values = values.filter(v => v.key !== entrySelected?.key);
            }
        }
    }

    const kvColumns: FeaturedColumnDef<KVValueResponse>[] = [
        { accessorKey: 'key', header: ta.kv.key, size: 220, meta: { dataType: 'text', filter: { type: 'text' } } },
        {
            id: 'value',
            header: ta.kv.value,
            size: 360,
            accessorFn: (e: KVValueResponse) => stringifyJsonValue(e.value),
            meta: { dataType: 'text', filter: { type: 'text' } },
        },
        {
            id: 'encrypted',
            header: ta.kv.storeEncrypted,
            size: 130,
            accessorFn: (e: KVValueResponse) => (e.encrypted ? '🔒' : ''),
        },
    ];
</script>

{#snippet error()}
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}
{/snippet}

{#snippet rowActions(entry: KVValueResponse)}
    <div class="flex items-center">
        {#if ns.public}
            <Button invisible onclick={() => openPubLink(entry.key)} ariaLabel="Public Access Link">
                <span title="Public Access Link"><IconLink width="1.1rem" /></span>
            </Button>
        {/if}
        <Button ariaLabel={ta.common.edit} invisible onclick={() => editEntry(entry)}>
            <IconEdit width="1.2rem" />
        </Button>
        <Button ariaLabel={t.common.delete} invisible level={-3} onclick={() => deleteEntry(entry)}>
            <IconTrash width="1.2rem" />
        </Button>
    </div>
{/snippet}

<div class="grid gap-3">
    <div class="flex flex-wrap items-end gap-2">
        <div class="w-32">
            <Input typ="number" bind:value={limit} label="Limit" min="1" max="4294967295" />
        </div>
        <div class="flex flex-wrap gap-2">
            <Button level={2} onclick={fetchAll}>{ta.kv.loadAllValues}</Button>
            <Button onclick={() => (showModalAdd = true)}>{ta.kv.addNewValue}</Button>
        </div>
    </div>

    <SearchBar bind:value={search} />

    {@render error()}

    <DataTable
        data={valuesSearch.length > 0 ? valuesSearch : values.length > 0 && search.length < 2 ? values : []}
        columns={kvColumns}
        clientSide
        rowActions={rowActions}
        rowActionsWidth={ns.public ? 132 : 92}
        initialPageSize={10}
        getRowId={e => e.key}
        emptyText={ta.common.noEntries}
    />

    <Modal bind:showModal={showModalAdd} bind:closeModal={closeModalAdd}>
        <Form action={urlValues} onSubmit={onSubmitAdd}>
            <Input
                bind:ref={refInput}
                name="key"
                label={ta.kv.key}
                placeholder={ta.kv.key}
                pattern={PATTERN_KV_KEY}
                required
            />
            <InputArea
                name="value"
                label={ta.kv.value}
                placeholder={ta.kv.value}
                rows={12}
                fontMono
                required
            />
            <InputCheckbox ariaLabel={ta.kv.storeEncrypted} name="encrypted" bind:checked={encryptNew}>
                {ta.kv.storeEncrypted}
            </InputCheckbox>
            {#if encryptNew}
                <p class="text-muted-foreground text-sm" transition:slide={{ duration: 150 }}>
                    {ta.kv.encryptedDesc}
                </p>
            {/if}

            <div class="flex gap-2">
                <Button type="submit">{t.common.save}</Button>
                <Button level={3} onclick={() => closeModalAdd?.()}>
                    {t.common.cancel}
                </Button>
            </div>

            {@render error()}
        </Form>
    </Modal>

    <Modal
        bind:showModal={showModalEdit}
        bind:closeModal={closeModalEdit}
        onClose={() => (entrySelected = undefined)}
    >
        {#if entrySelected}
            <Form action={urlValues} onSubmit={onSubmitEdit}>
                <h3 class="text-lg font-semibold break-all">
                    {entrySelected.key}
                </h3>
                <InputArea
                    bind:ref={refArea}
                    name="value"
                    label={ta.kv.value}
                    placeholder={ta.kv.value}
                    rows={12}
                    fontMono
                    value={stringifyJsonValue(entrySelected.value)}
                    required
                />
                <InputCheckbox name="encrypted" ariaLabel={ta.kv.storeEncrypted} checked={!!entrySelected.encrypted}>
                    {ta.kv.storeEncrypted}
                </InputCheckbox>
                {#if entrySelected.encrypted}
                    <p class="text-muted-foreground text-sm" transition:slide={{ duration: 150 }}>
                        {ta.kv.encryptedDesc}
                    </p>
                {/if}

                <div class="flex gap-2">
                    <Button type="submit">{t.common.save}</Button>
                    <Button level={3} onclick={() => closeModalEdit?.()}>
                        {t.common.cancel}
                    </Button>
                </div>
            </Form>
        {/if}
    </Modal>

    <Modal
        bind:showModal={showModalDelete}
        bind:closeModal={closeModalDelete}
        onClose={() => (entrySelected = undefined)}
    >
        {#if entrySelected}
            <div class="grid gap-4">
                <h3 class="text-lg font-semibold">
                    {t.common.delete}
                </h3>
                <p class="text-sm break-all">
                    {ta.kv.deleteConfirmMsg.replace('{{ key }}', entrySelected.key)}
                </p>
                <div class="flex gap-2">
                    <Button level={-1} onclick={onSubmitDelete}>{t.common.delete}</Button>
                    <Button level={3} onclick={() => closeModalDelete?.()}>
                        {t.common.cancel}
                    </Button>
                </div>
            </div>
        {/if}
    </Modal>
</div>
