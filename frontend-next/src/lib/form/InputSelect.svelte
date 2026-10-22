<script lang="ts">
    import * as Select from '$lib/components/ui/select/index.js';
    import { Label } from '$lib/components/ui/label';
    import { genKey } from '$utils/helpers';

    // A plain single choice from a fixed list. Empty = no value (offered unless `required`).
    let {
        id = genKey(),
        name,
        value = $bindable(),
        label = '',
        items,
        required = false,
        disabled = false,
        onChange,
    }: {
        id?: string;
        name?: string;
        value?: string | null;
        label?: string;
        items: { value: string; label: string }[];
        required?: boolean;
        disabled?: boolean;
        onChange?: (value: string) => void;
    } = $props();

    let current = $derived(items.find(i => i.value === value));
</script>

<div class="grid w-full gap-1.5">
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <input type="hidden" {name} value={value ?? ''} />
    <Select.Root
        type="single"
        value={value ?? ''}
        onValueChange={v => {
            value = v;
            onChange?.(v);
        }}
        {disabled}
    >
        <Select.Trigger {id} class="w-full" aria-label={label}>
            <span class={current ? '' : 'text-muted-foreground'}>{current?.label ?? '—'}</span>
        </Select.Trigger>
        <Select.Content>
            {#if !required}
                <Select.Item value="" label="—">—</Select.Item>
            {/if}
            {#each items as i (i.value)}
                <Select.Item value={i.value} label={i.label}>{i.label}</Select.Item>
            {/each}
        </Select.Content>
    </Select.Root>
</div>
