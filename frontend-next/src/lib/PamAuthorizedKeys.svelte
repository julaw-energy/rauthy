<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import type { PamSshAuthKeyRequest, PamSshAuthKeyResponse } from '$api/types/pam';
    import Button from '$lib/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Modal from '$lib/Modal.svelte';
    import InputArea from '$lib/form/InputArea.svelte';
    import Form from '$lib/form/Form.svelte';
    import { fetchDelete, fetchPost } from '$api/fetch';
    import LabeledValue from '$lib/LabeledValue.svelte';
    import { formatDateFromTs } from '$utils/helpers';
    import IconTrash from '$icons/IconTrash.svelte';
    import Tooltip from '$lib/Tooltip.svelte';

    let {
        authorizedKeys,
        onSave,
        isAdmin,
        pamUid,
    }: {
        authorizedKeys: undefined | PamSshAuthKeyResponse[];
        onSave: () => void;
        isAdmin?: boolean;
        pamUid?: number;
    } = $props();

    $inspect(isAdmin, pamUid).with(() => {
        if (isAdmin && !pamUid) {
            console.error('PamAuthorizedKeys must have pamUid given when isAdmin');
        }
    });

    const now = new Date().getTime() / 1000;

    let t = useI18n();

    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    let error = $state('');

    $effect(() => {
        if (error) {
            setTimeout(() => {
                error = '';
            }, 5000);
        }
    });

    async function deleteKey(tsAdded: number) {
        let url = '';
        if (isAdmin) {
            url = `/auth/v1/pam/users/${pamUid}/authorized_key/${tsAdded}`;
        } else {
            url = `/auth/v1/pam/users/self/authorized_keys/${tsAdded}`;
        }

        let res = await fetchDelete(url);
        if (res.error) {
            error = res.error.message;
        } else {
            onSave();
        }
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        let payload: PamSshAuthKeyRequest = {
            data: params.get('key')?.trim() || '',
        };

        let res = await fetchPost(form.action, payload);
        if (res.error) {
            error = res.error.message;
        } else {
            onSave();
            closeModal?.();
        }
    }
</script>

{#if authorizedKeys}
    <div class="grid gap-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
            <h4 class="text-base font-semibold">SSH Keys</h4>
            {#if !isAdmin}
                <Button level={2} onclick={() => (showModal = true)}>
                    {t.account.pam.addSshKey}
                </Button>
            {/if}
        </div>

        <Modal bind:showModal bind:closeModal class="sm:max-w-3xl">
            <h3 class="text-lg font-semibold">{t.account.pam.addSshKey}</h3>
            <Form action="/auth/v1/pam/users/self/authorized_keys" {onSubmit}>
                <InputArea
                    name="key"
                    label={t.account.pam.addSshKey}
                    placeholder="ssh-... <key> <comment>"
                    rows={10}
                    maxLength={8192}
                    required
                    fontMono
                />

                <div class="flex items-center gap-3">
                    <Button type="submit">{t.common.save}</Button>
                    <Msg err={error} />
                </div>
            </Form>
        </Modal>

        {#each authorizedKeys as key (key.ts_added)}
            <div class="bg-card grid gap-3 rounded-xl border p-4">
                <div class="flex items-start justify-between gap-2">
                    <div class="grid flex-1 gap-3 sm:grid-cols-2">
                        <LabeledValue label={t.account.pam.keyAdded}>
                            {formatDateFromTs(key.ts_added)}
                        </LabeledValue>
                        {#if key.expires}
                            <LabeledValue label={t.account.accessExp}>
                                <span class={now > key.expires ? 'text-destructive' : ''}>
                                    {formatDateFromTs(key.expires)}
                                </span>
                            </LabeledValue>
                        {/if}
                        <LabeledValue label={t.account.pam.keyType}>
                            {key.typ}
                        </LabeledValue>
                        <LabeledValue label={t.account.pam.comment}>
                            {key.comment}
                        </LabeledValue>
                    </div>
                    <Tooltip text={t.common.delete}>
                        <Button onclick={() => deleteKey(key.ts_added)} invisible level={-3} ariaLabel={t.common.delete}>
                            <IconTrash width="1.2rem" />
                        </Button>
                    </Tooltip>
                </div>
                <LabeledValue label="Key" mono>
                    <span class="break-all whitespace-normal">{key.data}</span>
                </LabeledValue>
            </div>
        {/each}
    </div>
{/if}
