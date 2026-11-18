<script lang="ts">
    import type { Component } from 'svelte';
    import { Check } from '@lucide/svelte';
    import * as Select from '$lib/components/ui/select';
    import Modal from '$lib/Modal.svelte';
    import { Button } from '$lib/components/ui/button';
    import { phone } from '$lib/isPhone.svelte';

    // A picker for the header: a select on a wide screen, and on a phone only its icon, which opens a
    // popup with the choices (a drop down list is hard to hit and often runs off the screen there).
    let {
        icon: Icon,
        ariaLabel,
        options,
        value,
        onChange,
        label,
        class: className = '',
    }: {
        icon: Component<{ class?: string }>;
        ariaLabel: string;
        options: { value: string; label: string }[];
        value: string;
        onChange: (value: string) => void;
        /** what the trigger shows next to the icon on a wide screen */
        label: string;
        class?: string;
    } = $props();

    let open = $state(false);
</script>

{#if phone.value}
    <Button variant="ghost" size="icon" aria-label={ariaLabel} title={ariaLabel} onclick={() => (open = true)}>
        <Icon class="size-5" />
    </Button>
    <Modal bind:showModal={open} class="max-w-xs! sm:max-w-xs">
        <h3 class="text-base font-semibold">{ariaLabel}</h3>
        <div class="grid gap-1">
            {#each options as o (o.value)}
                <button
                    type="button"
                    class="hover:bg-accent flex min-h-10 items-center justify-between gap-3 rounded-2xl px-3 text-left text-sm font-medium {o.value === value
                        ? 'bg-accent'
                        : ''}"
                    onclick={() => {
                        onChange(o.value);
                        open = false;
                    }}
                >
                    <span>{o.label}</span>
                    {#if o.value === value}<Check class="size-4" />{/if}
                </button>
            {/each}
        </div>
    </Modal>
{:else}
    <Select.Root type="single" {value} onValueChange={(v) => v && onChange(v)}>
        <Select.Trigger class="w-auto gap-2 {className}" aria-label={ariaLabel} title={ariaLabel}>
            <Icon class="text-muted-foreground size-4" />
            {label}
        </Select.Trigger>
        <Select.Content>
            {#each options as o (o.value)}
                <Select.Item value={o.value} label={o.label} class="pr-6">{o.label}</Select.Item>
            {/each}
        </Select.Content>
    </Select.Root>
{/if}
