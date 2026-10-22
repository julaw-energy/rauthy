<script lang="ts">
    import type { Snippet } from 'svelte';
    import { cn } from '$lib/utils';

    let {
        selected,
        onclick,
        children,
    }: {
        selected: boolean;
        pictureLeft?: boolean;
        onclick: () => void;
        children: Snippet;
    } = $props();
</script>

<li aria-current={selected ? 'page' : 'false'} class="list-none">
    <button
        type="button"
        {onclick}
        class={cn('hover:bg-accent w-full rounded-xl px-3 py-2 text-left text-sm', selected && 'bg-accent font-medium')}
    >
        {@render children()}
    </button>
</li>
