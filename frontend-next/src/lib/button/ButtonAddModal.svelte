<script lang="ts">
    import { type Snippet } from 'svelte';
    import { Plus } from '@lucide/svelte';
    import Button from '$lib5/button/Button.svelte';
    import Modal from '$lib5/Modal.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';

    let {
        alignRight,
        level = 2,
        closeModal = $bindable(),
        onClose,
        ref = $bindable(),
        children,
    }: {
        alignRight?: boolean;
        level?: number;
        closeModal?: undefined | (() => void);
        onClose?: () => void;
        ref?: undefined | HTMLButtonElement;
        children: Snippet;
    } = $props();

    let ta = useI18nAdmin();
    let showModal = $state(false);
</script>

<div class:ml-auto={alignRight}>
    <Button bind:ref {level} onclick={() => (showModal = true)}>
        <Plus class="size-4" />
        {ta.common.addNew}
    </Button>
    <Modal bind:showModal bind:closeModal {onClose}>
        {@render children()}
    </Modal>
</div>
