<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import type { IParam } from '$state/param.svelte';
    import { fetchGet, fetchPost } from '$api/fetch';
    import type { KVAccessRequest, KVAccessResponse } from '$api/types/kv';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Button from '$lib/button/Button.svelte';
    import Modal from '$lib/Modal.svelte';
    import Form from '$lib/form/Form.svelte';
    import Expandable from '$lib/Expandable.svelte';
    import KVAccessDetails from '$lib/admin/kv/KVAccessDetails.svelte';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import Input from '$lib/form/Input.svelte';
    import { PATTERN_GROUP } from '$utils/patterns';
    import KVAccessHelp from './KVAccessHelp.svelte';

    let {
        ns,
    }: {
        ns: IParam;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();

    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let keys: KVAccessResponse[] = $state([]);

    let urlAccess = $derived(`/auth/v1/kv/ns/${ns.get()}/access`);

    $effect(() => {
        fetchData();
    });

    $effect(() => {
        if (ref) {
            requestAnimationFrame(() => {
                ref?.focus();
            });
        }
    });

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        let payload: KVAccessRequest = {
            enabled: params.get('enabled') === 'on',
            name: params.get('name') || undefined,
        };
        let res = await fetchPost<KVAccessResponse>(form.action, payload);
        if (res.body) {
            keys.push(res.body);
            closeModal?.();
        } else {
            err = res.error?.message || 'error adding key';
        }
    }

    async function fetchData() {
        let res = await fetchGet<KVAccessResponse[]>(urlAccess);
        if (res.body) {
            keys = res.body;
        } else {
            err = res.error?.message || 'Fetch error';
        }
    }

    function onDelete(id: string) {
        keys = keys.filter(k => k.id !== id);
    }
</script>

<div class="grid gap-4">
    <Msg {err} />

    <div class="flex flex-wrap items-center gap-2">
        <Button onclick={() => (showModal = true)}>{ta.kv.addNewKey}</Button>
        <KVAccessHelp />
    </div>

    <Modal bind:showModal bind:closeModal>
        <Form action={urlAccess} {onSubmit}>
            <h3 class="text-lg font-semibold">{ta.kv.addNewKey}</h3>

            <InputCheckbox ariaLabel={ta.common.enabled} name="enabled" checked>
                {ta.common.enabled}
            </InputCheckbox>

            <Input
                bind:ref
                name="name"
                label={ta.common.name}
                placeholder={ta.common.name}
                value=""
                pattern={PATTERN_GROUP}
            />

            <div class="flex gap-2">
                <Button type="submit">
                    {t.common.save}
                </Button>
                <Button level={3} onclick={() => closeModal?.()}>
                    {t.common.cancel}
                </Button>
            </div>
        </Form>
    </Modal>

    <div class="grid gap-2">
        {#if keys.length === 0}
            <p class="text-muted-foreground text-sm">{ta.common.noEntries}</p>
        {:else}
            {#each keys as key, i (key.id)}
                <div class="rounded-xl border p-1">
                    <Expandable>
                        {#snippet summary()}
                            {#if key.name}
                                {key.name}
                            {:else}
                                <span class="font-mono">{key.id}</span>
                            {/if}
                        {/snippet}
                        {#snippet details()}
                            <KVAccessDetails bind:key={keys[i]} {urlAccess} {onDelete} />
                        {/snippet}
                    </Expandable>
                </div>
            {/each}
        {/if}
    </div>
</div>
