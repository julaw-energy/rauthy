<script lang="ts">
    import { Check, ChevronDown, X } from '@lucide/svelte';
    import * as Popover from '$lib/components/ui/popover';
    import * as Command from '$lib/components/ui/command';
    import { Label } from '$lib/components/ui/label';
    import { cn } from '$lib/utils';
    import { genKey } from '$utils/helpers';

    // Several values at once, shown as chips. With `items` they are picked from a searchable list, without
    // them (`free`) they are typed, each ended with Enter or a comma.
    let {
        id = genKey(),
        value = $bindable([]),
        label = '',
        placeholder = '',
        items,
        disabled = false,
    }: {
        id?: string;
        value?: string[];
        label?: string;
        placeholder?: string;
        /** what can be picked, `prefix` is shown before the label (a flag for example) */
        items?: { value: string; label: string; prefix?: string }[];
        disabled?: boolean;
    } = $props();

    let open = $state(false);
    let typed = $state('');

    const labelOf = (v: string) => items?.find(i => i.value === v);

    function toggle(v: string) {
        value = value.includes(v) ? value.filter(x => x !== v) : [...value, v];
    }

    function remove(v: string) {
        value = value.filter(x => x !== v);
    }

    function addTyped() {
        const parts = typed
            .split(',')
            .map(s => s.trim())
            .filter(s => s.length > 0 && !value.includes(s));
        if (parts.length > 0) value = [...value, ...parts];
        typed = '';
    }

    function onKey(ev: KeyboardEvent) {
        if (ev.key === 'Enter' || ev.key === ',') {
            ev.preventDefault();
            addTyped();
        } else if (ev.key === 'Backspace' && typed === '' && value.length > 0) {
            value = value.slice(0, -1);
        }
    }
</script>

<div class="grid w-full gap-1.5">
    {#if label}
        <Label for={id}>{label}</Label>
    {/if}

    {#if items}
        <Popover.Root bind:open>
            <Popover.Trigger
                {id}
                {disabled}
                aria-label={label || placeholder}
                class="border-input bg-white dark:bg-input/50 dark:border-transparent flex min-h-9 w-full items-center justify-between gap-2 rounded-3xl border px-3 py-1 text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/30 disabled:opacity-50"
            >
                <span class="flex min-w-0 flex-wrap items-center gap-1">
                    {#each value as v (v)}
                        {@const it = labelOf(v)}
                        <span class="bg-muted inline-flex items-center gap-1 rounded-full py-0.5 pr-1 pl-2 text-xs">
                            {#if it?.prefix}<span class="leading-none">{it.prefix}</span>{/if}
                            <span class="max-w-40 truncate">{it?.label ?? v}</span>
                            <span
                                role="button"
                                tabindex="-1"
                                aria-label="Remove {it?.label ?? v}"
                                class="hover:bg-background/60 inline-flex size-4 items-center justify-center rounded-full"
                                onclick={ev => {
                                    ev.stopPropagation();
                                    remove(v);
                                }}
                                onkeydown={() => {}}
                            >
                                <X class="size-3" />
                            </span>
                        </span>
                    {/each}
                    {#if value.length === 0}
                        <span class="text-muted-foreground">{placeholder || '—'}</span>
                    {/if}
                </span>
                <ChevronDown class="text-muted-foreground size-4 shrink-0" />
            </Popover.Trigger>
            <Popover.Content class="w-(--bits-popover-anchor-width) min-w-64 p-0" align="start">
                <Command.Root>
                    <Command.Input placeholder={placeholder || label} />
                    <Command.List class="max-h-64">
                        <Command.Empty>-</Command.Empty>
                        <Command.Group>
                            {#each items as it (it.value)}
                                <Command.Item value="{it.label} {it.value}" onSelect={() => toggle(it.value)}>
                                    <Check class={cn('size-4', !value.includes(it.value) && 'opacity-0')} />
                                    {#if it.prefix}<span class="text-base leading-none">{it.prefix}</span>{/if}
                                    <span class="min-w-0 flex-1 truncate">{it.label}</span>
                                    {#if it.label !== it.value}
                                        <span class="text-muted-foreground text-xs">{it.value}</span>
                                    {/if}
                                </Command.Item>
                            {/each}
                        </Command.Group>
                    </Command.List>
                </Command.Root>
            </Popover.Content>
        </Popover.Root>
    {:else}
        <div
            class="border-input bg-white dark:bg-input/50 dark:border-transparent focus-within:ring-ring/30 flex min-h-9 w-full flex-wrap items-center gap-1 rounded-3xl border px-3 py-1 text-sm focus-within:ring-3"
        >
            {#each value as v (v)}
                <span class="bg-muted inline-flex items-center gap-1 rounded-full py-0.5 pr-1 pl-2 text-xs">
                    <span class="max-w-40 truncate">{v}</span>
                    <button
                        type="button"
                        aria-label="Remove {v}"
                        class="hover:bg-background/60 inline-flex size-4 items-center justify-center rounded-full"
                        onclick={() => remove(v)}
                    >
                        <X class="size-3" />
                    </button>
                </span>
            {/each}
            <input
                {id}
                {disabled}
                bind:value={typed}
                onkeydown={onKey}
                onblur={addTyped}
                autocomplete="off"
                placeholder={value.length === 0 ? placeholder : ''}
                class="min-w-24 flex-1 bg-transparent py-0.5 outline-none"
            />
        </div>
    {/if}
</div>
