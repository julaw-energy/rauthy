<script lang="ts">
    import type { Snippet } from 'svelte';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';

    let {
        checked = $bindable(false),
        disabled,
        ariaLabel = '',
        name,
        children,
    }: {
        checked: boolean;
        disabled?: boolean;
        ariaLabel: string;
        borderColor?: string;
        name?: string;
        children?: Snippet;
    } = $props();
</script>

<Label class="inline-flex items-center gap-2 font-normal" data-disabled={disabled}>
    <Checkbox bind:checked {name} {disabled} aria-label={ariaLabel} />
    <span>{@render children?.()}</span>
</Label>
