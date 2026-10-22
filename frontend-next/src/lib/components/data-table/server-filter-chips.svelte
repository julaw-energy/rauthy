<script lang="ts" generics="TData extends RowData">
    import {X} from "@lucide/svelte";
    import type {RowData} from "@tanstack/svelte-table";
    import {Button} from "$lib/components/ui/button";
    import type {FeaturedTable} from "./data-table-features";
    import {describeFilter, getServerFilters} from "./server-filters";

    let {table}: { table: FeaturedTable<TData> } = $props();

    const server = getServerFilters();
    const chips = $derived(
        Object.entries(server?.applied ?? {}).map(([id, value]) => {
            const header = table.getColumn(id)?.columnDef.header;
            return {id, label: typeof header === "string" ? header : id, text: describeFilter(value)};
        }),
    );

    function remove(id: string) {
        server?.clear(id);
        table.getColumn(id)?.setFilterValue(undefined);
    }

    function removeAll() {
        for (const {id} of chips) table.getColumn(id)?.setFilterValue(undefined);
        server?.clearAll();
    }
</script>

{#if chips.length > 0}
    <div class="shrink-0 flex flex-wrap items-center gap-1 pb-2" aria-label="Active server filters">
        {#each chips as chip (chip.id)}
            <span class="inline-flex items-center gap-1 rounded-full border bg-muted px-2 py-0.5 text-xs">
                <span class="font-medium">{chip.label}</span> {chip.text}
                <button type="button" class="rounded-full hover:bg-black/10 dark:hover:bg-white/10"
                        aria-label={`Remove ${chip.label} filter`} onclick={() => remove(chip.id)}>
                    <X class="size-3"/>
                </button>
            </span>
        {/each}
        <Button variant="ghost" size="sm" class="h-6 px-2 text-xs" onclick={removeAll}>Clear all</Button>
    </div>
{/if}
