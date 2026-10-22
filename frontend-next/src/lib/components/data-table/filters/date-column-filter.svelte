<script lang="ts" generics="TData extends RowData">
    import DateTime from "@/components/date-time"
    import {type DateValue} from "@internationalized/date";
    import type {DateFilterValue} from "@/types/tanstack-table";
    import FilterAreaHeader from "./filter-area-header.svelte";
    import {initialDateFilterValue} from "./utils";
    import { Button } from "$lib/components/ui/button";
    import type {FeaturedColumn} from "../data-table-features";
    import type {RowData} from "@tanstack/svelte-table";

    let {
        column,
        type
    }: {
        column: FeaturedColumn<TData>; type?: "date" | "date-time"
    } = $props();

    const dateOnly = $derived(type === "date")

    // svelte-ignore state_referenced_locally
        const initial = (column.getFilterValue() as DateFilterValue | undefined) ??  {...initialDateFilterValue, "type": type}
    let startDate = $state<DateValue | undefined>(initial.from);
    let endDate = $state<DateValue | undefined>(initial.to);
    let showConfirm = $derived.by(()=>{
        if (startDate && endDate){
            if (startDate.compare(endDate)<= 0){
                return true
            }
        }
        return false
    })

    function filter(){
        column.setFilterValue({"type": type, from: startDate, to: endDate, values: []})
    }

</script>

<div class="flex flex-col gap-3">
    <FilterAreaHeader {column} />
    <!--{dateOnly}-->
    <DateTime bind:value={startDate} dateOnly={dateOnly} label="from:"/>
    <DateTime bind:value={endDate} dateOnly={dateOnly} label="to:"/>

    {#if showConfirm}
        <div class="flex justify-end">
            <Button
                    type="button"
                    size="sm"
                    class="dark:bg-brand-green dark:text-white rounded-md"
                    onclick={() => filter()}
            >
                Filter
            </Button>
        </div>
    {/if}
</div>