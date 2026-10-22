<script lang="ts">
    import type { Snippet } from 'svelte';
    import IconClipboard from '$icons/IconClipboard.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Tooltip from '$lib5/Tooltip.svelte';

    let {
        label,
        title,
        mono,
        copyToClip,
        button,
        children,
    }: {
        label?: string;
        title?: string;
        mono?: boolean;
        copyToClip?: string;
        button?: Snippet;
        children: Snippet;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let text = $state(t.common.copyToClip);

    function copy(value: string) {
        navigator.clipboard.writeText(value);
        text = ta.common.copiedToClip;
        setTimeout(() => {
            text = t.common.copyToClip;
        }, 3000);
    }
</script>

<div class="flex min-w-0 flex-col gap-1.5">
    {#if label}
        <div class="text-sm font-medium">{label}</div>
    {/if}
    <div class="flex items-center gap-1">
        <div
            title={title || label || ''}
            class="bg-muted/50 min-h-9 min-w-0 flex-1 truncate rounded-xl border px-3 py-1.5 text-sm {mono ? 'font-mono' : ''}"
        >
            {@render children()}
        </div>
        {#if button}
            {@render button()}
        {/if}
        {#if copyToClip}
            <Button invisible onclick={() => copy(copyToClip)}>
                <Tooltip {text}>
                    <IconClipboard width="1.1rem" />
                </Tooltip>
            </Button>
        {/if}
    </div>
</div>
