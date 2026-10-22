<script lang="ts">
    import Button from '$lib5/button/Button.svelte';
    import type { AuthProviderTemplate } from '$api/templates/AuthProvider.ts';

    let {
        ariaLabel,
        provider,
        onclick,
        isLoading,
    }: {
        ariaLabel: string;
        provider: AuthProviderTemplate;
        onclick: (providerId: string) => void;
        isLoading: boolean;
    } = $props();

    let showIcon = $state(false);
</script>

<Button {ariaLabel} level={2} onclick={() => onclick(provider.id)} {isLoading} class="w-full">
    <img
        src={`/auth/v1/providers/${provider.id}/img?updated=${provider.updated}`}
        alt="Provider Logo"
        width="20"
        height="20"
        class="size-5"
        class:hidden={!showIcon}
        aria-hidden={!showIcon}
        onload={() => (showIcon = true)}
    />
    <span>{provider.name}</span>
</Button>
