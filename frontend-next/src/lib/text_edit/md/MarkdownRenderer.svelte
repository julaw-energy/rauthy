<script lang="ts">
    import { useMarkdownWorker } from '$lib/text_edit/useWorker.svelte';

    let {
        markdown,
    }: {
        markdown?: string;
    } = $props();

    let worker = useMarkdownWorker();

    $effect(() => {
        if (markdown) {
            worker.renderMarkdown(markdown);
        }
    });
</script>

<div class="html-render prose prose-sm dark:prose-invert max-w-none">
    {@html worker.renderedMarkdown()}
</div>
