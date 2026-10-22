<script lang="ts" generics="TData extends RowData">
    import {Button} from "$lib/components/ui/button";
    import {Globe} from "@lucide/svelte";
    import type {RowData} from "@tanstack/svelte-table";
    import type {FilterValue} from "@/types/tanstack-table";
    import type {FeaturedColumn} from "../data-table-features";
    import {getServerFilters} from "../server-filters";

    let {column}: { column: FeaturedColumn<TData> } = $props();

    const server = getServerFilters();
    const local = $derived(column.getFilterValue() as FilterValue | undefined);
    const applied = $derived(server?.applied[column.id]);
    const sameAsApplied = $derived(!!local && !!applied && JSON.stringify(local) === JSON.stringify(applied));
    const canApply = $derived(!!local && !!server?.canApply(column.id, local));
</script>

{#if server && column.columnDef.meta?.serverFilter && (local || applied)}
    <div class="mt-2 flex items-center justify-between gap-2 border-t pt-2 text-xs text-muted-foreground">
        <span>
            {#if applied && sameAsApplied}Applied to all records
            {:else if applied}Server filter differs from this one
            {:else}Filtering loaded rows only{/if}
        </span>
        <div class="flex gap-1">
            {#if applied}
                <Button variant="ghost" size="sm" class="h-7 px-2" onclick={() => server.clear(column.id)}>
                    Clear fetch filter
                </Button>
            {/if}
            {#if local && !sameAsApplied}
                <Button size="sm" class="h-7 px-2" disabled={!canApply} onclick={() => server.apply(column.id, local)}
                        title={canApply ? undefined : "The backend can't apply this filter"}>
                    <Globe class="size-3"/> Search all
                </Button>
            {/if}
        </div>
    </div>
{/if}
