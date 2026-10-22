<script lang="ts">
    import * as Select from '$lib/components/ui/select/index.js';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';

    // Values match the backend (`RE_GENDER`): `female` | `male` | `other`; empty = no value.
    let {
        id = genKey(),
        name,
        value = $bindable(),
        label = '',
        required = false,
        disabled = false,
    }: {
        id?: string;
        name?: string;
        value?: string | null;
        label?: string;
        required?: boolean;
        disabled?: boolean;
    } = $props();

    let t = useI18n();

    let items = $derived([
        { value: 'female', label: t.account.genderFemale },
        { value: 'male', label: t.account.genderMale },
        { value: 'other', label: t.account.genderOther },
    ]);
    let current = $derived(items.find(i => i.value === value));
</script>

<div class="grid w-full gap-1.5">
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <input type="hidden" {name} value={value ?? ''} />
    <div class="relative">
        <Select.Root type="single" value={value ?? ''} onValueChange={v => (value = v)} {disabled}>
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
        {#if required}
            <!-- invisible proxy so native `required` validation still works -->
            <input
                tabindex="-1"
                aria-hidden="true"
                class="pointer-events-none absolute inset-0 opacity-0"
                required
                value={value ?? ''}
            />
        {/if}
    </div>
</div>
