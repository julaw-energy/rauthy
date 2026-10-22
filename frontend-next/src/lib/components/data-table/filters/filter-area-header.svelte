<script lang="ts" generics="TData extends RowData">
    import {FunnelX} from "@lucide/svelte";
    import {Button} from "@/components/ui/button";
    import type {FeaturedColumn} from "../data-table-features";
    import type {RowData} from "@tanstack/svelte-table";

    let {
        column
    }: {
        column: FeaturedColumn<TData>;
    } = $props();
</script>

<div class="flex items-center justify-between text-sm font-medium">
    <span>Filter <span class="underline italic">{column.columnDef.header ?? column.id}</span> </span>

    {#if column.getIsFiltered()}
        <Button
                variant="ghost"
                size="sm"
                class="px-2 w-fit text-destructive hover:bg-destructive/80 dark:hover:bg-destructive/50"
                onclick={() => column.setFilterValue(undefined)}
        >
            <FunnelX class="size-4" />
            Clear
        </Button>
    {/if}
</div>