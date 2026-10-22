<script lang="ts">
    import Tabs from '$lib/tabs/Tabs.svelte';
    import EditorMarkdownPreview from '$lib/text_edit/md/EditorMarkdownPreview.svelte';
    import EditorText from '$lib/text_edit/EditorText.svelte';
    import { useMarkdownWorker } from '$lib/text_edit/useWorker.svelte';
    import { onDestroy } from 'svelte';

    let {
        height = 'calc(90dvh - 5.5rem)',
        mode = $bindable('Markdown'),
        contentRaw = $bindable(''),
        sanitizedValue = $bindable(''),
    }: {
        height?: string;
        mode?: 'Text' | 'Markdown' | 'HTML';
        contentRaw?: string;
        sanitizedValue?: string;
    } = $props();

    let worker = useMarkdownWorker();

    onDestroy(() => {
        worker.closeWorker();
    });

    $effect(() => {
        if (mode === 'Text') {
            sanitizedValue = contentRaw;
        } else if (mode === 'Markdown') {
            sanitizedValue = worker.renderedMarkdown();
            // } else if (mode === 'HTML') {
            //     sanitizedValue = worker.sanitizedHTML();
        }
    });
</script>

<div class="grid gap-3">
    <Tabs bind:selected={mode} tabs={['Markdown', 'Text']} center />

    {#if mode === 'Markdown'}
        <EditorMarkdownPreview bind:content={contentRaw} {height} />
    {:else if mode === 'Text'}
        <EditorText bind:content={contentRaw} {height} hideButtons />
    {/if}
</div>
