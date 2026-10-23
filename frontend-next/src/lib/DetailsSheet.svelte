<script lang="ts">
    import type { Snippet } from 'svelte';
    import * as Sheet from '$lib/components/ui/sheet';
    import { ScrollArea } from '$lib/components/ui/scroll-area';

    // Right-hand drawer used by every list page to show the selected item (deep-linkable by the
    // page via its `?uid=` / `?id=` style param, like the old UI's side-by-side detail pane).
    let {
        open = false,
        title = '',
        onClose,
        wide = false,
        children,
    }: {
        open?: boolean;
        title?: string;
        onClose?: () => void;
        wide?: boolean;
        children: Snippet;
    } = $props();
</script>

<Sheet.Root
    {open}
    onOpenChange={(o) => {
        if (!o) onClose?.();
    }}
>
    <Sheet.Content side="right" class="flex flex-col gap-0 p-0 data-[side=right]:w-full data-[side=right]:sm:max-w-xl {wide ? 'data-[side=right]:lg:max-w-4xl' : 'data-[side=right]:lg:max-w-2xl'}">
        <Sheet.Header class="border-b p-4 pr-12">
            <Sheet.Title class="truncate">{title}</Sheet.Title>
        </Sheet.Header>
        <ScrollArea class="min-h-0 flex-1">
            <div class="p-4">
                {@render children()}
            </div>
        </ScrollArea>
    </Sheet.Content>
</Sheet.Root>
