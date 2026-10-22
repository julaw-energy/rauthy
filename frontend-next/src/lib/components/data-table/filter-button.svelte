<script lang="ts" generics="TData extends RowData">
    import { Funnel } from "@lucide/svelte/icons";
    import * as Popover from "$lib/components/ui/popover";

    import TextColumnFilter from "./filters/text-column-filter.svelte";
    import DateRangeColumnFilter from "./filters/date-column-filter.svelte";
    import NumberColumnFilter from "./filters/number-column-filter.svelte";
    import BooleanColumnFilter from "./filters/boolean-column-filter.svelte";
    import ServerFilterFooter from "./filters/server-filter-footer.svelte";
    import {getServerFilters} from "./server-filters";
    import type {FeaturedColumn} from "./data-table-features";
    import type {RowData} from "@tanstack/svelte-table";
    let {
        column
    }: {
        column: FeaturedColumn<TData>;
    } = $props();

    const columnType = $derived(column.columnDef.meta?.dataType ?? "text");
    const isFiltered = $derived(column.getIsFiltered());
    const filterIndex = $derived(column.getFilterIndex());
    const serverApplied = $derived(!!getServerFilters()?.applied[column.id]);
</script>

<Popover.Root>
    <Popover.Trigger>
        {#snippet child({ props })}
            <button
                    {...props}
                    type="button"
                    class="relative {isFiltered?'size-7':''} {serverApplied?'ring-2 ring-primary':''} items-center flex gap-1 rounded-sm text-left hover:bg-black/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring dark:hover:bg-white/10"
                    aria-label={`Filter ${column.id}${serverApplied ? " (applied to all records)" : ""}`}
            >
                {#if isFiltered}
                    <Funnel class="size-3" />
                    <span
                            class="
                              absolute right-0.5 top-0.5
                              flex size-3.5 items-center justify-center
                              rounded-full bg-primary
                              text-[9px] font-semibold text-primary-foreground
                            "
                                            aria-label={`Filter priority ${filterIndex + 1}`}
                                    >
                            {filterIndex + 1}
                    </span>
                {:else}
                    <Funnel class="size-3 opacity-40 group-hover:opacity-80" />
                {/if}
            </button>
        {/snippet}
    </Popover.Trigger>

    <Popover.Content
            align="start"
            class="w-80 [@media(max-height:500px)]:max-h-64 p-3 overflow-y-auto scrollbar-thin"
    >
        {#if columnType === "text"}
            <TextColumnFilter {column}/>
        {:else if columnType === "date" || columnType === "date-time"}
            <DateRangeColumnFilter {column} type={columnType}/>
        {:else if columnType === "number"}
            <NumberColumnFilter {column} />
        {:else if columnType === "boolean"}
            <BooleanColumnFilter {column} />
        {/if}
        <ServerFilterFooter {column}/>
    </Popover.Content>
</Popover.Root>