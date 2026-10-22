<script lang="ts">
    import { Search } from '@lucide/svelte';
    import { Input } from '$lib/components/ui/input';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';

    let t = useI18n();

    let {
        ref = $bindable(),
        id = genKey(),
        ariaLabel = t.common.search,
        value = $bindable(''),
        placeholder = t.common.search,
        datalist,
        pattern,
        focus = $bindable(),
        width = '100%',
        onSearch,
        onTab,
        onUp,
        onDown,
        onFocus,
    }: {
        ref?: undefined | HTMLElement;
        id?: string;
        ariaLabel?: string;
        value?: string;
        placeholder?: string;
        datalist?: string[];
        options?: string[];
        option?: string;
        pattern?: string;
        focus?: () => void;
        width?: string;
        borderless?: boolean;
        onSearch?: (value: string, option?: string) => void;
        onTab?: (value: string) => void;
        onUp?: (value: string) => void;
        onDown?: (value: string) => void;
        onFocus?: () => void;
    } = $props();

    let inner: HTMLInputElement | null = $state(null);
    $effect(() => {
        ref = inner ?? undefined;
        focus = () => inner?.focus();
    });
    const listId = `${id}-list`;

    function onkeydown(ev: KeyboardEvent) {
        switch (ev.code) {
            case 'Enter':
                onSearch?.(value);
                break;
            case 'Tab':
                onTab?.(value);
                break;
            case 'ArrowUp':
                onUp?.(value);
                break;
            case 'ArrowDown':
                onDown?.(value);
                break;
        }
    }
</script>

<div class="relative w-full" style:max-width={width !== '100%' ? width : undefined}>
    <Search class="text-muted-foreground absolute top-1/2 left-2.5 size-4 -translate-y-1/2" />
    <Input
        bind:ref={inner}
        {id}
        class="pl-8"
        type="search"
        aria-label={ariaLabel}
        {placeholder}
        {pattern}
        list={datalist && datalist.length > 0 ? listId : undefined}
        bind:value
        {onkeydown}
        onfocus={() => onFocus?.()}
    />
    {#if datalist && datalist.length > 0}
        <datalist id={listId}>
            {#each datalist as d (d)}
                <option value={d}></option>
            {/each}
        </datalist>
    {/if}
</div>
