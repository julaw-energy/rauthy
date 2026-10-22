<script lang="ts">
    import RauthyLogo from '$lib5/RauthyLogo.svelte';

    let {
        clientId,
        updated,
    }: {
        clientId: string;
        updated: undefined | number;
    } = $props();

    let showDefault = $state(false);
    let src = $derived(
        updated
            ? `/auth/v1/clients/${clientId}/logo?updated=${updated}`
            : `/auth/v1/clients/${clientId}/logo`,
    );
</script>

<div class="size-16 shrink-0 overflow-hidden rounded-xl">
    {#if showDefault}
        <RauthyLogo width="100%" />
    {:else}
        <img {src} alt="Client Logo" class="h-full w-full object-contain" onerror={() => (showDefault = true)} />
    {/if}
</div>
