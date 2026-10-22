<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Input from '$lib/form/Input.svelte';
    import type { UserValuesPreferredUsername } from '$api/templates/UserValuesConfig';
    import { useI18n } from '$state/i18n.svelte';
    import Button from '$lib/button/Button.svelte';
    import Modal from '$lib/Modal.svelte';
    import { fetchPut } from '$api/fetch';
    import type { PreferredUsernameRequest } from '$api/types/user';
    import { slide } from 'svelte/transition';
    import Tooltip from '$lib/Tooltip.svelte';
    import IconInfo from '$icons/IconInfo.svelte';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import Form from '$lib/form/Form.svelte';

    let {
        userId,
        preferred_username = $bindable(),
        config,
        isAdmin,
        isGroupAdmin,
    }: {
        userId: string;
        preferred_username: string | undefined;
        config: UserValuesPreferredUsername;
        isAdmin?: boolean;
        // a delegated group admin may only set the username while it is still
        // empty; overwriting an existing one is reserved for full admins
        isGroupAdmin?: boolean;
    } = $props();

    let t = useI18n();

    let ref: undefined | HTMLInputElement = $state();
    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    let error = $state('');
    let username = $state('');
    let forceOverwrite = $state(false);
    let nameExists = $state(false);
    let required = $derived(config.preferred_username === 'required');

    let isEmpty = $derived(!preferred_username || preferred_username.length === 0);
    // who may open the editor: a full admin always; a group admin only while empty; the
    // user itself while empty or when the value is not immutable
    let canEdit = $derived(isAdmin || (isGroupAdmin ? isEmpty : isEmpty || !config.immutable));

    $effect(() => {
        username = preferred_username || '';
        forceOverwrite = false;
    });

    $effect(() => {
        if (ref) {
            requestAnimationFrame(() => {
                ref?.focus();
            });
        }
    });

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (preferred_username && config.immutable && !forceOverwrite) {
            return;
        }

        if (config.blacklist.includes(username)) {
            nameExists = true;
            return;
        }

        let payload: PreferredUsernameRequest = {
            preferred_username: username || undefined,
            force_overwrite: forceOverwrite || undefined,
        };
        let res = await fetchPut(form.action, payload);
        if (res.status === 200) {
            closeModal?.();
            preferred_username = username;
            forceOverwrite = false;
        } else if (res.status === 406) {
            nameExists = true;
        } else if (res.error) {
            error = res.error.message;
        }
    }
</script>

{#if config.preferred_username !== 'hidden'}
    <div class="grid gap-1.5">
        <div class="text-sm font-medium">
            {t.account.preferredUsername.preferredUsername}{#if required}<span class="text-destructive">*</span>{/if}
        </div>

        <div class="flex items-center justify-between gap-2 rounded-xl border px-3 py-1.5 {required && !preferred_username ? 'border-destructive' : ''}">
            <div class="min-w-0 truncate">
                {preferred_username || '-'}
            </div>

            <div class="shrink-0">
                {#if canEdit}
                    <Button
                        ariaLabel={t.common.edit}
                        level={required && !preferred_username ? -1 : 3}
                        onclick={() => (showModal = true)}
                    >
                        {t.common.edit}
                    </Button>

                    <Modal bind:showModal bind:closeModal>
                        <h3 class="text-lg font-semibold">{t.account.preferredUsername.preferredUsername}</h3>

                        <Form
                            action={`/auth/v1/users/${userId}/self/preferred_username`}
                            {onSubmit}
                        >
                            <Input
                                bind:ref
                                name="preferred_username"
                                autocomplete="off"
                                label={t.account.preferredUsername.preferredUsername}
                                placeholder={t.account.preferredUsername.preferredUsername}
                                bind:value={username}
                                {required}
                                isError={nameExists}
                                errMsg={nameExists ? t.account.preferredUsername.notAvailable : ''}
                                pattern={config.pattern_html}
                                onInput={() => (nameExists = false)}
                            />

                            <div class="text-muted-foreground grid gap-1 text-sm">
                                <p>{t.account.preferredUsername.desc}</p>
                                {#if config.immutable}
                                    <p class="text-destructive">
                                        {t.account.preferredUsername.immutable}
                                    </p>
                                {/if}
                            </div>

                            {#if config.immutable && isAdmin}
                                <InputCheckbox
                                    ariaLabel={t.account.preferredUsername.forceOverwrite}
                                    bind:checked={forceOverwrite}
                                >
                                    {t.account.preferredUsername.forceOverwrite}
                                </InputCheckbox>
                            {/if}

                            {#if !preferred_username || !config.immutable || (forceOverwrite && isAdmin)}
                                <div class="flex gap-2" transition:slide={{ duration: 150 }}>
                                    <Button type="submit">
                                        {t.common.save}
                                    </Button>
                                    <Button level={-3} onclick={() => closeModal?.()}>
                                        {t.common.cancel}
                                    </Button>
                                </div>

                                <Msg err={error} />
                            {/if}
                        </Form>
                    </Modal>
                {:else}
                    <Tooltip text={t.account.preferredUsername.immutableInfo}>
                        <IconInfo width="1.2rem" />
                    </Tooltip>
                {/if}
            </div>
        </div>
    </div>
{/if}
