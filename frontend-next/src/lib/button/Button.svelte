<script lang="ts">
    import type { Snippet } from 'svelte';
    import { Button, type ButtonVariant } from '$lib/components/ui/button';
    import { Spinner } from '$lib/components/ui/spinner';

    // Same props as the old UI's Button. `level`: 1 primary, 2 outline, 3 ghost,
    // negative levels are the destructive counterparts.
    let {
        type = 'button',
        role = 'button',
        ref = $bindable(),
        id,
        ariaLabel,
        ariaControls,
        ariaCurrent,
        level = 1,
        width,
        isDisabled = false,
        isLoading = false,
        invisible = false,
        invisibleOutline = false,
        popovertarget,
        popovertargetaction,
        onclick,
        onTab,
        onLeft,
        onRight,
        onUp,
        onDown,
        children,
        class: className,
        ...rest
    }: {
        type?: 'button' | 'submit' | 'reset' | null | undefined;
        role?: string;
        ref?: undefined | HTMLButtonElement;
        id?: string;
        ariaLabel?: string;
        ariaControls?: string;
        ariaCurrent?: 'time' | 'page' | 'step' | 'location' | 'date' | undefined;
        level?: number;
        width?: string;
        isDisabled?: boolean;
        isLoading?: boolean;
        invisible?: boolean;
        invisibleOutline?: boolean;
        popovertarget?: string;
        popovertargetaction?: 'toggle' | 'show' | 'hide' | null | undefined;
        children: Snippet;
        class?: string;

        onclick?: (ev: Event) => void;
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

    let variant: ButtonVariant = $derived.by(() => {
        if (invisible) return 'ghost';
        switch (level) {
            case -1:
                return 'destructive';
            case -2:
            case -3:
                return 'destructive';
            case 2:
                return 'outline';
            case 3:
                return 'ghost';
            default:
                return 'default';
        }
    });

    let extra = $derived(
        level === -2 ? 'bg-transparent text-destructive border-destructive/40 hover:text-destructive' : level === -3 ? 'bg-transparent text-destructive hover:text-destructive' : '',
    );

    function onkeydown(ev: KeyboardEvent) {
        switch (ev.code) {
            case 'Tab':
                onTab?.();
                break;
            case 'ArrowLeft':
                onLeft?.();
                break;
            case 'ArrowRight':
                onRight?.();
                break;
            case 'ArrowUp':
                onUp?.();
                break;
            case 'ArrowDown':
                onDown?.();
                break;
        }
    }
</script>

<Button
    bind:ref={inner}
    {type}
    {role}
    {id}
    {variant}
    size={invisible ? 'icon-sm' : 'default'}
    aria-label={ariaLabel}
    aria-controls={ariaControls}
    aria-current={ariaCurrent}
    class="{extra} {invisible ? 'h-auto w-auto p-1' : ''} {className ?? ''}"
    style={width ? `width:${width}` : undefined}
    disabled={isDisabled || isLoading}
    {popovertarget}
    {popovertargetaction}
    {onclick}
    {onkeydown}
    {...rest}
>
    {#if isLoading}
        <Spinner class="size-4" />
    {:else}
        {@render children()}
    {/if}
</Button>
