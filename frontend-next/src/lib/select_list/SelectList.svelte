<script lang="ts">
    import type { Snippet } from 'svelte';
    import type { SelectItem } from '$lib5/select_list/props.ts';
    import Button from '$lib5/button/Button.svelte';
    import IconEdit from '$icons/IconEdit.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Modal from '$lib5/Modal.svelte';
    import InputCheckbox from '$lib5/form/InputCheckbox.svelte';
    import { Badge } from '$lib/components/ui/badge';

    let {
        items = $bindable(),
        maxWidth = '467pt',
        disabledItems = [],
        children,
    }: {
        items: SelectItem[];
        maxWidth?: string;
        // names of items that are rendered read-only and cannot be toggled
        disabledItems?: string[];
        children: Snippet;
    } = $props();

    let ta = useI18nAdmin();

    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    let selected = $derived(items.filter((i) => i.selected));
    // when every entry is locked there is nothing to edit, so hide the edit button
    let readonly = $derived(items.length > 0 && items.every((i) => disabledItems.includes(i.name)));
</script>

<div class="flex w-full items-start gap-2" style:max-width={maxWidth}>
    <div class="flex min-h-9 flex-1 flex-wrap items-center gap-1.5">
        {#each selected as item (item.name)}
            <Badge variant="secondary">{item.name}</Badge>
        {/each}
    </div>

    {#if !readonly}
        <Button invisible onclick={() => (showModal = true)}>
            <span title={ta.common.edit}><IconEdit width="1.2rem" /></span>
        </Button>
    {/if}

    <Modal bind:showModal bind:closeModal>
        <h3 class="text-lg font-semibold">{@render children()}</h3>
        <div class="grid max-h-[60dvh] grid-cols-1 gap-2 overflow-y-auto sm:grid-cols-2">
            {#each items as item (item.name)}
                <InputCheckbox ariaLabel={item.name} bind:checked={item.selected} disabled={disabledItems.includes(item.name)}>
                    {item.name}
                </InputCheckbox>
            {/each}
        </div>
        <div class="flex justify-end">
            <Button onclick={() => closeModal?.()}>OK</Button>
        </div>
    </Modal>
</div>
