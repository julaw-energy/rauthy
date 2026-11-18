<script lang="ts">
    import type { Snippet } from 'svelte';
    import * as Dialog from '$lib/components/ui/dialog';

    let {
        showModal = $bindable(false),
        closeModal = $bindable(),
        onClose,
        strict = false,
        children,
        class: className,
    }: {
        showModal?: boolean;
        closeModal?: () => void;
        onClose?: () => void;
        strict?: boolean;
        prerender?: boolean;
        children: Snippet;
        class?: string;
    } = $props();

    $effect(() => {
        closeModal = () => (showModal = false);
    });
</script>

<Dialog.Root
    bind:open={showModal}
    onOpenChange={(open) => {
        if (!open) onClose?.();
    }}
>
    <Dialog.Content
        showCloseButton={!strict}
        interactOutsideBehavior={strict ? 'ignore' : 'close'}
        escapeKeydownBehavior={strict ? 'ignore' : 'close'}
        class="max-h-[90dvh] max-w-[calc(100vw-1rem)] gap-4 overflow-x-hidden overflow-y-auto p-4 sm:max-w-xl sm:gap-6 sm:p-6 [&>*]:min-w-0 {className ?? ''}"
    >
        {@render children()}
    </Dialog.Content>
</Dialog.Root>
