<script lang="ts">
    import { tick } from 'svelte';
    import { Search, X } from '@lucide/svelte';
    import { Input } from '$lib/components/ui/input';
    import { Button } from '$lib/components/ui/button';
    import { phone } from '$lib/isPhone.svelte';

    // A search box that is only an icon on a phone, until it is tapped. It stays open while it has a text.
    let {
        value = $bindable(''),
        placeholder = 'Search',
        expanded = $bindable(true),
        class: className = '',
    }: {
        value?: string;
        placeholder?: string;
        /** whether the input is shown, for a toolbar that needs to make room for it */
        expanded?: boolean;
        class?: string;
    } = $props();

    let opened = $state(false);
    let input: HTMLInputElement | null = $state(null);

    $effect(() => {
        expanded = !phone.value || opened || !!value;
    });

    async function show() {
        opened = true;
        await tick();
        input?.focus();
    }

    function hide() {
        value = '';
        opened = false;
    }
</script>

{#if expanded}
    <div class="relative min-w-0 {className}">
        <Search class="text-muted-foreground absolute top-1/2 left-2.5 size-4 -translate-y-1/2" />
        <Input class="h-8 pl-8 {phone.value ? 'pr-8' : ''}" {placeholder} aria-label={placeholder} bind:ref={input} bind:value />
        {#if phone.value}
            <button
                type="button"
                class="text-muted-foreground absolute top-1/2 right-2 -translate-y-1/2"
                aria-label="Close search"
                onclick={hide}
            >
                <X class="size-4" />
            </button>
        {/if}
    </div>
{:else}
    <Button variant="outline" size="icon-sm" aria-label={placeholder} title={placeholder} onclick={show}>
        <Search class="size-4" />
    </Button>
{/if}
