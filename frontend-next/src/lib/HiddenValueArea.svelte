<script lang="ts">
    import IconClipboard from '$icons/IconClipboard.svelte';
    import IconEye from '$icons/IconEye.svelte';
    import IconEyeSlash from '$icons/IconEyeSlash.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { Textarea } from '$lib/components/ui/textarea';
    import { useI18n } from '$state/i18n.svelte';

    let {
        ariaLabel = '',
        value = '',
        rows = 10,
        show = $bindable(false),
        width = 'min(25rem, calc(100dvw - 2rem))',
    }: {
        ariaLabel: string;
        value: string;
        rows?: number;
        cols?: number;
        show?: boolean;
        width?: string;
    } = $props();

    let t = useI18n();
    let text = $derived(show ? value : '*'.repeat(value.length));

    function copyToClip() {
        navigator.clipboard.writeText(value);
    }
</script>

<div class="relative" style:max-width={width} style:width="100%">
    <div class="absolute top-1 right-1 z-10 flex flex-col">
        <Button ariaLabel={show ? t.common.hide : t.common.show} invisible onclick={() => (show = !show)}>
            <span title={show ? t.common.hide : t.common.show}>
                {#if show}<IconEye width="1.1rem" />{:else}<IconEyeSlash width="1.1rem" />{/if}
            </span>
        </Button>
        <Button ariaLabel={t.common.copyToClip} invisible onclick={copyToClip}>
            <span title={t.common.copyToClip}><IconClipboard width="1.1rem" /></span>
        </Button>
    </div>
    <Textarea aria-label={ariaLabel} class="pr-12 font-mono" disabled value={text} {rows} />
</div>
