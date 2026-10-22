<script lang="ts">
    import * as Select from '$lib/components/ui/select';
    import * as Popover from '$lib/components/ui/popover';
    import * as Command from '$lib/components/ui/command';
    import { cn } from '$lib/utils';
    import { Check, ChevronDown } from '@lucide/svelte';

    let {
        ref = $bindable(),
        ariaLabel,
        options = [],
        name,
        value = $bindable(),
        maxHeight = '16rem',
        borderless = false,
        withSearch = false,
        onChange,
        onLeft,
        onRight,
        onUp,
        onDown,
    }: {
        ref?: undefined | HTMLButtonElement;
        ariaLabel: string;
        options: string[] | number[];
        name?: string;
        value?: string | number;
        maxHeight?: string;
        offsetTop?: string;
        offsetLeft?: string;
        anchored?: boolean;
        borderless?: boolean;
        withSearch?: boolean;
        onChange?: (value: string | number | undefined) => void;
        onLeft?: () => void;
        onRight?: () => void;
        onUp?: () => void;
        onDown?: () => void;
    } = $props();

    // shadcn refs have a `null` fallback; `bind:ref` of the wrapper may start as undefined
    let inner: HTMLButtonElement | null = $state(null);
    $effect(() => {
        ref = inner ?? undefined;
    });

    // bits-ui selects work on strings, the old UI also allowed numbers
    let strValue = $derived(value === undefined || value === null ? '' : String(value));
    let isNumber = $derived(options.length > 0 && typeof options[0] === 'number');

    function change(v: string) {
        const next = isNumber ? Number(v) : v;
        value = next;
        onChange?.(next);
        open = false;
    }

    let open = $state(false);
</script>

{#if withSearch}
    <input type="hidden" {name} value={strValue} />
    <Popover.Root bind:open>
        <Popover.Trigger
            bind:ref={inner}
            aria-label={ariaLabel}
            class={cn(
                'border-input bg-white dark:bg-input/50 dark:border-transparent flex h-9 w-full min-w-24 items-center justify-between gap-1.5 rounded-3xl border px-3 py-2 text-sm whitespace-nowrap outline-none focus-visible:ring-3 focus-visible:ring-ring/30',
                borderless && 'border-transparent bg-transparent shadow-none',
            )}
        >
            <span class="truncate">{strValue || ariaLabel}</span>
            <ChevronDown class="text-muted-foreground size-4 shrink-0" />
        </Popover.Trigger>
        <Popover.Content class="w-(--bits-popover-anchor-width) min-w-56 p-0" align="start">
            <Command.Root>
                <Command.Input placeholder={ariaLabel} />
                <Command.List style="max-height:{maxHeight}">
                    <Command.Empty>-</Command.Empty>
                    <Command.Group>
                        {#each options as o (o)}
                            <Command.Item value={String(o)} onSelect={() => change(String(o))}>
                                <Check class={cn('size-4', String(o) !== strValue && 'opacity-0')} />
                                {o}
                            </Command.Item>
                        {/each}
                    </Command.Group>
                </Command.List>
            </Command.Root>
        </Popover.Content>
    </Popover.Root>
{:else}

<Select.Root type="single" {name} value={strValue} onValueChange={change}>
    <Select.Trigger
        bind:ref={inner}
        aria-label={ariaLabel}
        class={cn('w-full min-w-24', borderless && 'border-transparent bg-transparent shadow-none')}
        onkeydown={(ev: KeyboardEvent) => {
            if (ev.code === 'ArrowLeft') onLeft?.();
            else if (ev.code === 'ArrowRight') onRight?.();
            else if (ev.code === 'ArrowUp') onUp?.();
            else if (ev.code === 'ArrowDown') onDown?.();
        }}
    >
        {strValue || ariaLabel}
    </Select.Trigger>
    <Select.Content style="max-height:{maxHeight}">
        {#each options as o (o)}
            <Select.Item value={String(o)} label={String(o)}>{o}</Select.Item>
        {/each}
    </Select.Content>
</Select.Root>
{/if}
