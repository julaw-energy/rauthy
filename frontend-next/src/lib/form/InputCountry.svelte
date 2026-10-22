<script lang="ts">
    import { Check, ChevronDown, X } from '@lucide/svelte';
    import * as Popover from '$lib/components/ui/popover';
    import * as Command from '$lib/components/ui/command';
    import { Label } from '$lib/components/ui/label';
    import { cn } from '$lib/utils';
    import { genKey } from '$utils/helpers';
    import { countryList, flag, resolveCountry } from '$lib5/countries';

    // Stores the ISO 3166-1 alpha-2 code (`KE`). Legacy free-text values ("Kenya") are mapped to
    // their code when shown; text that matches no country is kept untouched until a new one is picked.
    let {
        id = genKey(),
        name,
        value = $bindable(),
        label = '',
        placeholder = '',
        required = false,
        disabled = false,
        onChange,
    }: {
        id?: string;
        name?: string;
        value?: string | null;
        label?: string;
        placeholder?: string;
        required?: boolean;
        disabled?: boolean;
        onChange?: (code: string) => void;
    } = $props();

    let open = $state(false);
    const countries = countryList();

    let code = $derived(resolveCountry(value));
    let current = $derived(countries.find(c => c.code === code));

    // normalize legacy text to the code once the form has a value
    $effect(() => {
        if (code && value !== code) value = code;
    });

    function pick(c: string) {
        value = c;
        open = false;
        onChange?.(c);
    }
</script>

<div class="grid w-full gap-1.5">
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <input type="hidden" {name} value={value ?? ''} />
    <Popover.Root bind:open>
        <div class="relative">
            <Popover.Trigger
                {id}
                {disabled}
                aria-label={label || placeholder}
                class="border-input bg-white dark:bg-input/50 dark:border-transparent flex h-9 w-full items-center justify-between gap-2 rounded-3xl border px-3 text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/30 disabled:opacity-50"
            >
                <span class={cn('flex min-w-0 items-center gap-2', !value && 'text-muted-foreground')}>
                    {#if current}
                        <span class="text-base leading-none">{flag(current.code)}</span>
                        <span class="truncate">{current.name}</span>
                    {:else}
                        <span class="truncate">{value || placeholder}</span>
                    {/if}
                </span>
                <ChevronDown class="text-muted-foreground size-4 shrink-0" />
            </Popover.Trigger>
            <!-- invisible proxy so native `required` validation still works -->
            {#if required}
                <input
                    tabindex="-1"
                    aria-hidden="true"
                    class="pointer-events-none absolute inset-0 opacity-0"
                    required
                    value={value ?? ''}
                />
            {/if}
        </div>
        <Popover.Content class="w-(--bits-popover-anchor-width) min-w-64 p-0" align="start">
            <Command.Root>
                <Command.Input placeholder={placeholder || label} />
                <Command.List class="max-h-64">
                    <Command.Empty>-</Command.Empty>
                    <Command.Group>
                        {#if !required && value}
                            <Command.Item value="clear" onSelect={() => ((value = ''), (open = false))}>
                                <X class="size-4" /> —
                            </Command.Item>
                        {/if}
                        {#each countries as c (c.code)}
                            <Command.Item value="{c.name} {c.code}" onSelect={() => pick(c.code)}>
                                <Check class={cn('size-4', c.code !== code && 'opacity-0')} />
                                <span class="text-base leading-none">{flag(c.code)}</span>
                                <span class="min-w-0 flex-1 truncate">{c.name}</span>
                                <span class="text-muted-foreground text-xs">{c.code}</span>
                            </Command.Item>
                        {/each}
                    </Command.Group>
                </Command.List>
            </Command.Root>
        </Popover.Content>
    </Popover.Root>
</div>
