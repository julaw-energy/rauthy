<script lang="ts">
    import type { Snippet } from 'svelte';
    import Button from '$lib/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';

    let {
        content = $bindable(''),
        ref = $bindable(),
        hideButtons = false,
        height = '40rem',
        // append a custom, non-editable text at the bottom
        appendCustomText,
        onCancel,
        onSave,
        onKeyUp,
        onMoueUp,
        onFocus,
        onFocusOut,
        onScroll,
        children,
    }: {
        content: string;
        ref?: undefined | HTMLElement;
        hideButtons?: boolean;
        height?: string;
        appendCustomText?: string;
        onCancel?: () => void;
        onSave?: () => void;
        onKeyUp?: () => void;
        onMoueUp?: () => void;
        onFocus?: () => void;
        onFocusOut?: () => void;
        onScroll?: () => void;
        children?: Snippet;
    } = $props();

    let t = useI18n();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div role="none" class="grid gap-2" style:height onclick={() => ref?.focus()}>
    {@render children?.()}

    <div
        bind:this={ref}
        id="editor"
        role="textbox"
        tabindex="0"
        class="border-input bg-background focus-visible:ring-ring/30 focus-visible:border-ring min-h-48 overflow-auto rounded-xl border p-3 font-mono text-sm whitespace-pre-wrap outline-none focus-visible:ring-3"
        contenteditable="plaintext-only"
        bind:innerText={content}
        onkeyup={onKeyUp}
        onmouseup={onMoueUp}
        onfocus={onFocus}
        onfocusout={onFocusOut}
        onscroll={onScroll}
    ></div>

    {#if appendCustomText}
        <div class="bg-muted/40 overflow-auto rounded-xl border p-3">
            <pre class="text-xs">{appendCustomText}</pre>
        </div>
    {/if}

    {#if !hideButtons}
        <div class="flex gap-2">
            <Button onclick={onSave}>
                {t.common.save}
            </Button>
            <Button level={3} onclick={onCancel}>
                {t.common.cancel}
            </Button>
        </div>
    {/if}
</div>
