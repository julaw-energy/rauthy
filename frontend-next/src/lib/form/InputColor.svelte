<script lang="ts">
    import { Label } from '$lib/components/ui/label';

    let {
        name,
        value = $bindable(),
        label = '',
        disabled = false,
    }: {
        name?: string;
        value: string;
        label: string;
        disabled?: boolean | null | undefined;
    } = $props();

    const id = `color-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="grid gap-1.5">
    <Label for={id}>{label}</Label>
    <input {id} {name} type="color" bind:value {disabled} class="border-input h-9 w-16 cursor-pointer rounded-xl border bg-transparent p-1 disabled:opacity-50" />
</div>
