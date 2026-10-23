<script lang="ts">
    import Button from '$lib5/button/Button.svelte';
    import Msg from '$lib5/Msg.svelte';
    import { useI18n } from '$state/i18n.svelte';

    // Footer of the "add new" modals. Both buttons submit the form; `another` tells the caller which
    // one was used (true = keep the modal open with an empty form for the next entry).
    let {
        another = $bindable(false),
        err = '',
        isLoading = false,
    }: {
        another?: boolean;
        err?: string;
        isLoading?: boolean;
    } = $props();

    let t = useI18n();
</script>

<div class="bg-background sticky bottom-0 -mx-1 mt-2 grid gap-2 border-t px-1 py-3">
    {#if err}
        <Msg {err} />
    {/if}
    <div class="flex flex-wrap items-center gap-2">
        <Button type="submit" {isLoading} onclick={() => (another = false)}>
            {t.common.save}
        </Button>
        <Button type="submit" level={2} {isLoading} onclick={() => (another = true)}>
            {t.common.save} &amp; add another
        </Button>
    </div>
</div>
