<script lang="ts">
    import { type Snippet } from 'svelte';

    let { children, top = false }: { children: Snippet; top?: boolean } = $props();
</script>

<!-- `top`: keep the content at the top, so a page whose height changes (tabs) does not move around -->
<div class="flex w-full flex-1 flex-col items-center {top ? 'justify-start' : 'justify-center'} gap-4 p-2 sm:p-4">
    {@render children()}
</div>
