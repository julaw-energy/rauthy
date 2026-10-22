<script lang="ts">
    import { Check, Ban } from '@lucide/svelte';

    let { checked = $bindable() }: { checked?: boolean } = $props();
</script>

{#if checked}
    <Check class="text-primary size-5" />
{:else}
    <Ban class="text-destructive size-5" />
{/if}
