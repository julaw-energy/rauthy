<script lang="ts">
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Tabs from '$lib5/tabs/Tabs.svelte';
    import type { ClientResponse } from '$api/types/clients.ts';
    import ClientConfig from '$lib5/admin/clients/ClientConfig.svelte';
    import ClientSecret from '$lib5/admin/clients/ClientSecret.svelte';
    import ClientDelete from '$lib5/admin/clients/ClientDelete.svelte';
    import ClientBranding from '$lib5/admin/clients/branding/ClientBranding.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import AuditSummary from '$lib5/admin/audit/AuditSummary.svelte';

    let {
        client,
        clients,
        scopesAll,
        onSave,
    }: {
        client: ClientResponse;
        clients: ClientResponse[];
        scopesAll: string[];
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    const tabs = [ta.nav.config, 'Secret', 'Branding', t.common.delete];
    let selected = $state(tabs[0]);

    let focusFirst: undefined | (() => void) = $state();

    // the client object is replaced after every save, only a different client moves the focus
    const clientId = $derived(client.id);

    $effect(() => {
        if (clientId) {
            requestAnimationFrame(() => {
                focusFirst?.();
            });
        }
    });
</script>

<Tabs {tabs} bind:selected bind:focusFirst />
<AuditSummary targetId={client.id} refreshKey={client} />

<div class="mt-4">
    {#if selected === ta.nav.config}
        <ClientConfig {client} {clients} {scopesAll} {onSave} />
    {:else if selected === 'Secret'}
        <ClientSecret {client} />
    {:else if selected === 'Branding'}
        <ClientBranding {client} />
    {:else if selected === t.common.delete}
        <ClientDelete {client} {onSave} />
    {/if}
</div>
