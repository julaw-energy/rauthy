<script lang="ts">
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete } from '$api/fetch';
    import type { ClientResponse } from '$api/types/clients.ts';

    let {
        client,
        onSave,
    }: {
        client: ClientResponse;
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let isLoading = $state(false);
    let err = $state('');

    async function onSubmit() {
        err = '';
        isLoading = true;

        let res = await fetchDelete(`/auth/v1/clients/${client.id}`);
        isLoading = false;

        if (res.error) {
            err = res.error.message;
        } else {
            onSave();
        }
    }
</script>

<div class="grid max-w-xl gap-4">
    <p class="text-muted-foreground text-sm">{ta.clients.delete1}</p>
    <div>
        <Button level={-1} onclick={onSubmit} {isLoading}>
            {t.common.delete}
        </Button>
    </div>

    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}
</div>
