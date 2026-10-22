<script lang="ts">
    import { genKey } from '$utils/helpers';
    import IconTrash from '$icons/IconTrash.svelte';
    import Button from '$lib5/button/Button.svelte';

    let {
        providerId,
        onDelete,
    }: {
        providerId: string;
        onDelete: () => void;
    } = $props();

    let exists = $state(false);
</script>

<div class="flex items-center gap-2">
    <img
        src={`/auth/v1/providers/${providerId}/img?${genKey()}}`}
        width={20}
        height={20}
        alt="-"
        onload={() => (exists = true)}
    />
    {#if exists}
        <Button invisible level={-3} onclick={onDelete}>
            <IconTrash width="1.1rem" />
        </Button>
    {/if}
</div>
