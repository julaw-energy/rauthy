<script lang="ts">
    import { type Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
</script>

<div class="flex w-full flex-1 flex-col items-center justify-center gap-4 p-2 sm:p-4">
    {@render children()}
</div>
