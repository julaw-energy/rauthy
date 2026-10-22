<script lang="ts">
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete } from '$api/fetch';
    import type { RoleResponse } from '$api/types/roles.ts';

    let {
        role,
        onSave,
    }: {
        role: RoleResponse;
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');
    let isRauthyAdmin = $derived(role.name === 'rauthy_admin');

    async function onSubmit() {
        err = '';

        let res = await fetchDelete(`/auth/v1/roles/${role.id}`);
        if (res.error) {
            err = res.error.message;
        } else {
            onSave();
        }
    }
</script>

<div class="grid max-w-xl gap-4">
    {#if isRauthyAdmin}
        <p class="text-muted-foreground text-sm">{@html ta.roles.adminNoMod}</p>
    {:else}
        <p class="text-muted-foreground text-sm">{ta.roles.delete1}</p>
        <div>
            <Button level={-1} onclick={onSubmit}>
                {t.common.delete}
            </Button>
        </div>

        {#if err}
            <p class="text-destructive text-sm">{err}</p>
        {/if}
    {/if}
</div>
