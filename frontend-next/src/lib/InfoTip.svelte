<script lang="ts">
    import { Info } from '@lucide/svelte';
    import * as Tooltip from '$lib/components/ui/tooltip';
    import * as Popover from '$lib/components/ui/popover';
    import { touch } from '$lib/isPhone.svelte';
    import { brand } from '$lib/appName.svelte';

    // A small (i) that explains something. With a mouse it shows on hover (and on keyboard focus).
    // A touch screen has no hover, so there it opens on a tap and closes on a tap elsewhere.
    let {
        text: raw,
        label = 'More information',
        html = false,
    }: { text: string; label?: string; /** the text holds trusted HTML (<b>, <code>) */ html?: boolean } = $props();
    // the texts are written with the product name, show the name this service is branded with
    let text = $derived(raw.replace(/\bRauthy\b/g, () => brand.name));
</script>

{#if touch.value}
    <Popover.Root>
        <Popover.Trigger
            class="text-muted-foreground hover:text-foreground inline-flex size-5 shrink-0 items-center justify-center rounded-full"
            aria-label={label}
        >
            <Info class="size-3.5" />
        </Popover.Trigger>
        <Popover.Content class="max-h-[60vh] w-72 overflow-y-auto p-3 text-xs leading-relaxed">
            <div class="block">{#if html}{@html text}{:else}{text}{/if}</div>
        </Popover.Content>
    </Popover.Root>
{:else}
    <Tooltip.Provider delayDuration={150}>
        <Tooltip.Root>
            <Tooltip.Trigger
                class="text-muted-foreground hover:text-foreground inline-flex size-5 shrink-0 items-center justify-center rounded-full"
                aria-label={label}
            >
                <Info class="size-3.5" />
            </Tooltip.Trigger>
            <Tooltip.Content class="max-w-80 text-xs leading-relaxed">
                <div class="block">{#if html}{@html text}{:else}{text}{/if}</div>
            </Tooltip.Content>
        </Tooltip.Root>
    </Tooltip.Provider>
{/if}
