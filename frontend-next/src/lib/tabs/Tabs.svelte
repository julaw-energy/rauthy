<script lang="ts">
    import * as Tabs from '$lib/components/ui/tabs';

    let {
        tabs,
        selected = $bindable(''),
        center = false,
        width = 'inherit',
        focusFirst = $bindable(),
        onDown,
    }: {
        tabs: string[];
        selected: string;
        borderRadius?: string;
        center?: boolean;
        width?: string;
        focusFirst?: () => void;
        onDown?: () => void;
    } = $props();

    let ref: undefined | HTMLElement = $state();

    $effect(() => {
        focusFirst = () => ref?.querySelector<HTMLElement>('[role=tab]')?.focus();
    });
</script>

<div bind:this={ref} class="max-w-full overflow-x-auto overflow-y-hidden {center ? 'flex justify-center' : ''}" style:width={width !== 'inherit' ? width : undefined}>
    <Tabs.Root bind:value={selected}>
        <Tabs.List>
            {#each tabs as tab (tab)}
                <Tabs.Trigger value={tab} onkeydown={(ev) => ev.code === 'ArrowDown' && onDown?.()}>{tab}</Tabs.Trigger>
            {/each}
        </Tabs.List>
    </Tabs.Root>
</div>
