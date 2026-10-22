<script lang="ts">
    import ToolbarIcon from '$lib/text_edit/md/ToolbarIconMarkdown.svelte';
    import IconBold from '$icons/editor/IconBold.svelte';
    import IconH1 from '$icons/editor/IconH1.svelte';
    import IconH2 from '$icons/editor/IconH2.svelte';
    import IconH3 from '$icons/editor/IconH3.svelte';
    import IconItalic from '$icons/editor/IconItalic.svelte';
    import IconStrikeThrough from '$icons/editor/IconStrikeThrough.svelte';
    import IconListBullet from '$icons/editor/IconListBullet.svelte';
    import IconListNumbered from '$icons/editor/IconListNumbered.svelte';
    import IconQueueList from '$icons/editor/IconQueueList.svelte';
    import IconCode from '$icons/editor/IconCode.svelte';
    import IconQuote from '$icons/editor/IconQuote.svelte';
    import IconLink from '$icons/editor/IconLink.svelte';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import MarkdownRenderer from '$lib/text_edit/md/MarkdownRenderer.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useMarkdownWorker } from '$lib/text_edit/useWorker.svelte';

    let {
        markdown = $bindable(),
        withPreview = false,
        onblur,
    }: {
        markdown: string;
        withPreview?: boolean;
        onblur?: () => void;
    } = $props();

    const iconSize = '1.3rem';

    let ta = useI18nAdmin();
    let worker = useMarkdownWorker();

    let ref: undefined | HTMLDivElement = $state();
    let preview = $state(false);

    $effect(() => {
        worker.renderMarkdown(markdown);
    });

    $effect(() => {
        if (!withPreview && preview) {
            preview = false;
        }
    });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div role="none" class="grid gap-2" onclick={() => ref?.focus()}>
    <div class="flex flex-wrap items-center justify-between gap-2">
        <div>
            {#if withPreview}
                <InputCheckbox ariaLabel={ta.common.preview} bind:checked={preview}>
                    {ta.common.preview}
                </InputCheckbox>
            {/if}
        </div>

        <div class="flex flex-wrap items-center">
            <ToolbarIcon {ref} title={ta.editor.heading1} bind:content={markdown} addLeft="# ">
                <IconH1 width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.heading2} bind:content={markdown} addLeft="## ">
                <IconH2 width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.heading3} bind:content={markdown} addLeft="### ">
                <IconH3 width={iconSize} />
            </ToolbarIcon>

            <ToolbarIcon {ref} title={ta.editor.bold} bind:content={markdown} addLeft="**" addRight="**">
                <IconBold width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.italic} bind:content={markdown} addLeft="*" addRight="*">
                <IconItalic width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.strikeThrough} bind:content={markdown} addLeft="~~" addRight="~~">
                <IconStrikeThrough width={iconSize} />
            </ToolbarIcon>

            <ToolbarIcon {ref} title={ta.editor.listBullet} bind:content={markdown} addLeft="- ">
                <IconListBullet width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.listNumbered} bind:content={markdown} addLeft="1. ">
                <IconListNumbered width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.listTasks} bind:content={markdown} addLeft="- [ ] ">
                <IconQueueList width={iconSize} />
            </ToolbarIcon>

            <ToolbarIcon {ref} title={ta.editor.code} bind:content={markdown} addLeft="`" addRight="`">
                <IconCode width={iconSize} />
            </ToolbarIcon>
            <ToolbarIcon {ref} title={ta.editor.quote} bind:content={markdown} addLeft="> ">
                <IconQuote width={iconSize} />
            </ToolbarIcon>

            <ToolbarIcon {ref} title={ta.editor.link} bind:content={markdown} addLeft="[" addRightLink="https://...">
                <IconLink width={iconSize} />
            </ToolbarIcon>
        </div>
    </div>

    {#if preview}
        <div class="bg-muted/30 min-h-48 overflow-auto rounded-xl border p-3">
            <MarkdownRenderer />
        </div>
    {:else}
        <div
            role="textbox"
            tabindex="0"
            bind:this={ref}
            class="border-input bg-background focus-visible:ring-ring/30 focus-visible:border-ring min-h-48 overflow-auto rounded-xl border p-3 font-mono text-sm whitespace-pre-wrap outline-none focus-visible:ring-3"
            aria-label={ta.editor.textArea}
            bind:innerText={markdown}
            contenteditable="plaintext-only"
            spellcheck="false"
            {onblur}
        ></div>
    {/if}
</div>
