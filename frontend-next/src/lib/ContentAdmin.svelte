<script lang="ts">
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
</script>

<div class="flex min-w-0 flex-1 flex-col gap-4">
    {@render children()}
</div>
