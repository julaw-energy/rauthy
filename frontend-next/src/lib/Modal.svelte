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
        class="max-h-[90dvh] overflow-y-auto sm:max-w-xl {className ?? ''}"
    >
        {@render children()}
    </Dialog.Content>
</Dialog.Root>
