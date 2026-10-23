<script lang="ts" generics="TData extends RowData">
    import {FlexRender, type RowData} from "@tanstack/svelte-table";
    import FilterButton from "./filter-button.svelte";
    import SortButton from "./sort-button.svelte";
    import type {FeaturedHeader} from "./data-table-features";

    let {
        header,
        title,
      shouldTruncate
    }: {
        header: FeaturedHeader<TData>;
        title: string;
      shouldTruncate: boolean;
    } = $props();
</script>

<div role="columnheader" aria-colspan={header.colSpan}
        class="group relative text-base font-bold dt-header dt-border border-r px-1 py-1"
        style="flex: {header.getSize()} 1 0px; min-width: {Math.min(header.getSize(), 110)}px;"
>
    {#if !header.isPlaceholder}
        <div class="min-w-0 flex items-center justify-between gap-2">
            <div class="min-w-0" class:truncate={shouldTruncate} title={shouldTruncate ? title : undefined}>
              <FlexRender header={header}/>
            </div>
            <div class="flex items-center gap-0">
                {#if header.column.getCanSort()}
                  <SortButton title={title} column={header.column}/>
                {/if}
                {#if header.column.getCanFilter()}
                    <FilterButton column={header.column}/>
                {/if}
            </div>
        </div>

        {#if header.column.getCanResize()}
            <div
                    role="button"
                    aria-label="Resize column"
                    tabindex="0"
                    class="absolute -right-1 top-0 z-40 h-full w-2 cursor-col-resize touch-none select-none bg-transparent"
                    onmousedown={header.getResizeHandler()}
                    ontouchstart={header.getResizeHandler()}
            ></div>
        {/if}
    {/if}
</div>