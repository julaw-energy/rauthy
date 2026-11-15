<script lang="ts">
    import { Check } from '@lucide/svelte';
    import ErrorAlert from '$lib5/ErrorAlert.svelte';

    // Shared inline feedback used by forms: an error text and/or a success check.
    let {
        err = '',
        success = false,
        text = '',
    }: {
        err?: string;
        success?: boolean;
        text?: string;
    } = $props();
</script>

{#if success}
    <span class="text-primary inline-flex items-center gap-1 text-sm" role="status">
        <Check class="size-5" />
        {text}
    </span>
{/if}
<ErrorAlert {err} />
