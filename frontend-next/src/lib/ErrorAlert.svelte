<script lang="ts">
    import { CircleAlert } from '@lucide/svelte';
    import type { Snippet } from 'svelte';

    // An error the user must notice: a bordered box with an icon, not a bare red line.
    // Give it the text as `err`, or put the content inside (for a message with more than a sentence).
    let {
        err = '',
        children,
        class: className = '',
    }: {
        err?: string;
        children?: Snippet;
        class?: string;
    } = $props();
</script>

{#if err || children}
    <div
        role="alert"
        class="border-destructive/50 bg-destructive/10 flex w-full items-start gap-2.5 rounded-lg border px-3 py-2.5 text-start text-sm {className}"
    >
        <CircleAlert class="text-destructive mt-0.5 size-4 shrink-0" aria-hidden="true" />
        <div class="min-w-0 break-words">
            {#if children}
                {@render children()}
            {:else}
                {err}
            {/if}
        </div>
    </div>
{/if}
