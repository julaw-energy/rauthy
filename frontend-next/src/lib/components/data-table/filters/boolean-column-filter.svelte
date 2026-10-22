<script lang="ts" generics="TData extends RowData">
    import FilterAreaHeader from "./filter-area-header.svelte";
    import * as RadioGroup from "$lib/components/ui/radio-group/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import {initialBooleanFilterValue} from "./utils";
    import type {BooleanFilterValue} from "@/types/tanstack-table";
    import type {FeaturedColumn} from "../data-table-features";
    import type {RowData} from "@tanstack/svelte-table";

    let {
        column
    }: {
        column: FeaturedColumn<TData>;
    } = $props();

    const current = $derived((column.getFilterValue() as BooleanFilterValue | undefined) ?? initialBooleanFilterValue);

    function onValueChange(value: string){
        if(value === "all"){
            column.setFilterValue(undefined)
        }else {
            column.setFilterValue({...current, selection: value});
        }
    }
</script>

<div class="space-y-1">
    <FilterAreaHeader {column} />

    <RadioGroup.Root value={current.selection} class="flex items-center gap-2" onValueChange={onValueChange}>
        <div class="flex items-center space-x-2">
            <RadioGroup.Item value="all" id="r1-all" />
            <Label for="r1-all">ALL</Label>
        </div>
        <div class="flex items-center space-x-2">
            <RadioGroup.Item value="true" id="r2-true" />
            <Label for="r2-true">TRUE</Label>
        </div>
        <div class="flex items-center space-x-2">
            <RadioGroup.Item value="false" id="r3-false" />
            <Label for="r3-false">FALSE</Label>
        </div>
    </RadioGroup.Root>
</div>
