<script lang="ts">
    import { type Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
</script>

<div class="bg-background text-foreground relative flex min-h-dvh w-full flex-col">
    {@render children()}
</div>
