<script lang="ts">
    import Button from '$lib5/button/Button.svelte';
    import type { ApiKeyResponse } from '$api/types/api_keys.ts';
    import { fetchDelete } from '$api/fetch';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';

    let {
        key,
        onSave,
    }: {
        key: ApiKeyResponse;
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');

    async function onSubmit() {
        err = '';

        let res = await fetchDelete(`/auth/v1/api_keys/${key.name}`);
        if (res.error) {
            err = res.error.message;
        } else {
            onSave();
        }
    }
</script>

<div class="grid max-w-xl gap-4">
    <p class="text-muted-foreground text-sm">{ta.api_key.delete1}</p>
    <div>
        <Button level={-1} onclick={onSubmit}>
            {t.common.delete}
        </Button>
    </div>

    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}
</div>
