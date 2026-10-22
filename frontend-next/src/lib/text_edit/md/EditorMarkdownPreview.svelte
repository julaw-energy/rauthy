<script lang="ts">
    import MarkdownRenderer from '$lib/text_edit/md/MarkdownRenderer.svelte';
    import EditorMarkdown from '$lib/text_edit/md/EditorMarkdown.svelte';

    let {
        content = $bindable(''),
        height = '50rem',
    }: {
        content: string;
        height?: string;
    } = $props();

    let innerWidth: undefined | number = $state(
        typeof window !== 'undefined' ? window.innerWidth : undefined,
    );
    let previewEmbedded = $derived(!!(innerWidth && innerWidth < 1440));
</script>

<svelte:window bind:innerWidth />

<div class="min-h-0" style:height>
    <div class="grid h-full gap-3 {previewEmbedded ? '' : 'lg:grid-cols-2'}">
        <div class="min-w-0">
            <EditorMarkdown bind:markdown={content} withPreview={previewEmbedded} />
        </div>

        {#if !previewEmbedded}
            <div class="bg-muted/30 min-w-0 overflow-auto rounded-xl border p-3">
                <i class="text-muted-foreground text-sm">Preview</i>
                <MarkdownRenderer />
            </div>
        {/if}
    </div>
</div>
