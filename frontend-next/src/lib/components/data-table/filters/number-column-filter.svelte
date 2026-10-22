<script lang="ts" generics="TData extends RowData">
    import { Input } from "$lib/components/ui/input";
    import * as Select from "$lib/components/ui/select";
    import type {NumberFilterValue, NumberFilterOperator} from "@/types/tanstack-table";
    import {allNumberFilterOperators, initialNumberFilterValue} from "./utils";
    import FacetedMultiSelectFilter, {type FacetedOption} from "./faceted-multi-select-filter.svelte";
    import FilterAreaHeader from "./filter-area-header.svelte";
    import {SvelteSet} from "svelte/reactivity";
    import type {FeaturedColumn} from "../data-table-features";
    import type {RowData} from "@tanstack/svelte-table";

    let {
        column
    }: {
        column: FeaturedColumn<TData>;
    } = $props();

    let operators = $derived.by(() => {
        if (column.columnDef.meta){
            if (column.columnDef.meta.filter){
                if (column.columnDef.meta.filter.type === "number"){
                    if (column.columnDef.meta.filter.operators){
                        return column.columnDef.meta.filter.operators
                    }
                }
            }
        }

        return allNumberFilterOperators;
    });

    const options = $derived.by<FacetedOption[]>(() => {
        return Array.from(column.getFacetedUniqueValues().entries())
            .filter(([value]) => value !== null && value !== undefined)
            .map(([value, count]) => ({
                value: String(value),
                label: String(value),
                count
            }))
            .sort((a, b) => a.label.localeCompare(b.label));
    });

    let currentOperator = $state(initialNumberFilterValue.operator);
    let betweenValues = $state({from: NaN, to: NaN});
    const strBetweenValues = $derived({from: Number.isNaN(betweenValues.from)?"":String(betweenValues.from), to: Number.isNaN(betweenValues.to)?"":String(betweenValues.to)});

    const selectedValues = $derived.by(() => {
        const filter = column.getFilterValue() as NumberFilterValue | undefined;
        return new SvelteSet((filter ?? initialNumberFilterValue).values.map((v)=> String(v)));
    });


    const current = $derived(
        (column.getFilterValue() as NumberFilterValue | undefined) ?? {...initialNumberFilterValue, operator: currentOperator}
    );

    const stringValue = $derived(Number.isNaN(current.value)?"":String(current.value))

    function updateOperator(operator: NumberFilterOperator) {
        currentOperator = operator;
        if(!Number.isNaN(current.value)) {
            column.setFilterValue({
                ...current,
                operator
            });
        }
    }

    function updateValue(value: string) {
        if (!value) {
            column.setFilterValue(undefined);
            return;
        }

        column.setFilterValue({
            ...current,
            value,
            operator: currentOperator
        });
    }

    function updateValues(values: string[]) {
        if (values.length === 0 || values.length === options.length){
            column.setFilterValue(undefined);
        }else {
            column.setFilterValue({
                ...current, operator: "in",
                values: values.map((v) => Number(v))
            });
        }
    }

    function updateBetweenValue(position: "from" | "to", value: string){
        const numVal = Number(value)
        betweenValues = {...betweenValues, [position]: numVal}
        if(Number.isNaN(betweenValues.from) || Number.isNaN(betweenValues.to)){
            column.setFilterValue(undefined);
        } else {
            column.setFilterValue({
                ...current, operator: "between",
                from: betweenValues.from, to: betweenValues.to
            });
        }
    }
</script>

<div class="space-y-1">
    <FilterAreaHeader {column} />

    {#if selectedValues.size === 0}
        <div class="flex items-center gap-2">
            <Select.Root type="single" value={currentOperator} onValueChange={(value) => {
                if (value) updateOperator(value as NumberFilterOperator);
            }}
            >
                <Select.Trigger>
                    {currentOperator}
                </Select.Trigger>

                <Select.Content>
                    {#each operators as operator, index (index)}
                        <Select.Item value={operator}>
                            {operator}
                        </Select.Item>
                    {/each}
                </Select.Content>
            </Select.Root>
            {#if current.operator === "between"}
                <Input
                        type="number"
                        value={strBetweenValues.from}
                        placeholder="1st"
                        oninput={(event) => updateBetweenValue("from", event.currentTarget.value)}
                />

                <span>and</span>

                <Input
                        type="number"
                        value={strBetweenValues.to}
                        placeholder="2nd"
                        oninput={(event) => updateBetweenValue("to", event.currentTarget.value)}
                />
                {:else }
                <Input
                        type="number"
                        value={stringValue}
                        placeholder="Enter filter number..."
                        oninput={(event) => updateValue(event.currentTarget.value)}
                />
            {/if}
        </div>
    {/if}

    {#if !(column.getIsFiltered() && current.operator !== "in")}
        <FacetedMultiSelectFilter selectedValues={selectedValues} updateValues={updateValues} options={options}/>
    {/if}
</div>
