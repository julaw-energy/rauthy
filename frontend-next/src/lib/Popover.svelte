<script lang="ts">
    import type { Snippet } from 'svelte';
    import * as Popover from '$lib/components/ui/popover';
    import { cn } from '$lib/utils';

    let {
        ref = $bindable(),
        ariaLabel,
        children,
        btnDisabled,
        btnInvisible,
        button,
        close = $bindable(),
        lazy,
        onToggle,
        onTab,
        onLeft,
        onRight,
        onUp,
        onDown,
    }: {
        ref?: undefined | HTMLButtonElement;
        ariaLabel: string;
        children: Snippet;
        roleButton?: string;
        offsetLeft?: string;
        offsetTop?: string;
        absolute?: boolean;
        anchored?: boolean;
        lazy?: boolean;
        btnDisabled?: boolean;
        btnInvisible?: boolean;
        button: Snippet;
        close?: () => void;

        onToggle?: (newState: 'open' | 'closed') => void;
        onTab?: () => void;
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

    let open = $state(false);

    $effect(() => {
        close = () => (open = false);
    });
</script>

<Popover.Root
    bind:open
    onOpenChange={(o) => onToggle?.(o ? 'open' : 'closed')}
>
    <Popover.Trigger
        bind:ref={inner}
        aria-label={ariaLabel}
        disabled={btnDisabled}
        class={cn(
            'inline-flex items-center gap-1 rounded-xl text-sm',
            btnInvisible ? 'hover:bg-accent p-1' : 'border-input hover:bg-accent border px-3 py-1.5',
        )}
        onkeydown={(ev: KeyboardEvent) => {
            if (ev.code === 'Tab') onTab?.();
            else if (ev.code === 'ArrowLeft') onLeft?.();
            else if (ev.code === 'ArrowRight') onRight?.();
            else if (ev.code === 'ArrowUp') onUp?.();
            else if (ev.code === 'ArrowDown') onDown?.();
        }}
    >
        {@render button()}
    </Popover.Trigger>
    <Popover.Content class="w-auto max-w-[calc(100vw-2rem)]">
        {#if !lazy || open}
            {@render children()}
        {/if}
    </Popover.Content>
</Popover.Root>
