<script lang="ts">
    import { Spinner } from '$lib/components/ui/spinner';

    let {
        background = false,
        global = false,
        offset = 0,
    }: {
        background?: boolean;
        color?: string;
        global?: boolean;
        offset?: number;
    } = $props();
</script>

<div
    class="flex items-center justify-center {global
        ? 'fixed inset-0 z-50 ' + (background ? 'bg-background/70' : '')
        : 'h-full w-full'}"
    style:margin-top={offset ? `${offset}px` : undefined}
>
    <Spinner class="size-6" />
</div>
