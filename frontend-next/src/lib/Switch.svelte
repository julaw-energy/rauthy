<script lang="ts">
    import type { Snippet } from 'svelte';
    import { Switch } from '$lib/components/ui/switch';
    import { Label } from '$lib/components/ui/label';

    let {
        checked = $bindable(false),
        disabled,
        ariaLabel = '',
        name = '',
        labelWidth,
        children,
    }: {
        checked: boolean;
        disabled?: boolean;
        ariaLabel: string;
        name?: string;
        labelWidth?: string;
        children: Snippet;
    } = $props();
</script>

<div class="flex items-center gap-3">
    <Label class="flex-1 font-normal sm:flex-none" style={labelWidth ? `min-width:${labelWidth}` : undefined}>
        {@render children()}
    </Label>
    <Switch bind:checked {name} {disabled} aria-label={ariaLabel} />
</div>
