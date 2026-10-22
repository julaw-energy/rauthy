<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { onMount } from 'svelte';
    import type { ProviderLinkedUserResponse, ProviderResponse } from '$api/types/auth_provider.ts';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import InputCheckbox from '$lib5/form/InputCheckbox.svelte';
    import Expandable from '$lib5/Expandable.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useI18n } from '$state/i18n.svelte';

    let {
        provider,
        onSave = $bindable(),
    }: {
        provider: ProviderResponse;
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let isLoading = $state(true);
    let err = $state('');
    let forceDelete = $state(false);
    let linkedUsers: ProviderLinkedUserResponse[] = $state([]);

    onMount(async () => {
        let res = await fetchGet(`/auth/v1/providers/${provider.id}/delete_safe`);

        // If there are any linked users, the backend will return 406 instead of 200
        // to make it as clear as possible, that it is NOT safe to delete.
        if (res.status === 406 && res.error) {
            linkedUsers = res.error as unknown as ProviderLinkedUserResponse[];
        }

        isLoading = false;
    });

    async function onSubmit() {
        err = '';
        isLoading = true;

        let res = await fetchDelete(`/auth/v1/providers/${provider.id}`);
        if (res.error) {
            err = res.error.message;
        } else {
            onSave();
        }

        isLoading = false;
    }
</script>

<div class="grid max-w-xl gap-4">
    {#if linkedUsers.length > 0}
        <p class="text-destructive font-semibold">{ta.providers.delete.isInUse1}</p>
        <p class="text-muted-foreground text-sm">{ta.providers.delete.isInUse2}</p>

        <div class="rounded-xl border p-1">
            <Expandable>
                {#snippet summary()}
                    {ta.providers.delete.linkedUsers}
                {/snippet}
                {#snippet details()}
                    <div class="grid max-h-60 gap-1 overflow-y-auto">
                        {#each linkedUsers as user (user.id)}
                            <div>
                                {user.email}
                                <span class="text-muted-foreground font-mono"> / {user.id}</span>
                            </div>
                        {/each}
                    </div>
                {/snippet}
            </Expandable>
        </div>

        <p class="text-sm">{ta.providers.delete.areYouSure}</p>
        <InputCheckbox ariaLabel={ta.providers.delete.forceDelete} bind:checked={forceDelete}>
            {ta.providers.delete.forceDelete}
        </InputCheckbox>

        {#if forceDelete}
            <div>
                <Button level={-1} onclick={onSubmit} {isLoading}>
                    {t.common.delete}
                </Button>
            </div>
        {/if}
    {:else}
        <p class="text-muted-foreground text-sm">{ta.providers.delete.areYouSure}</p>
        <div>
            <Button level={-1} onclick={onSubmit} {isLoading}>
                {t.common.delete}
            </Button>
        </div>
    {/if}
    <Msg {err} />
</div>
