<script lang="ts" generics="TData extends RowData">
    import { Input } from "$lib/components/ui/input";
    import * as Select from "$lib/components/ui/select";
    import type {TextFilterValue, TextFilterOperator} from "@/types/tanstack-table";
    import {allTextFilterOperators, initialTextFilterValue} from "./utils";
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
                if (column.columnDef.meta.filter.type === "text"){
                    if (column.columnDef.meta.filter.operators){
                        return column.columnDef.meta.filter.operators
                    }
                }
            }
        }

        return allTextFilterOperators;
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

    let currentOperator = $state(initialTextFilterValue.operator);
    const selectedValues = $derived.by(() => {
        const filter = column.getFilterValue() as TextFilterValue | undefined;
        return new SvelteSet((filter ?? initialTextFilterValue).values);
    });

    const current = $derived(
        (column.getFilterValue() as TextFilterValue | undefined) ?? {...initialTextFilterValue, operator: currentOperator}
    );

    function updateOperator(operator: TextFilterOperator) {
        currentOperator = operator;
        if(current.value.length>0) {
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
                values
            });
        }
    }
</script>

<div class="space-y-1">
    <FilterAreaHeader {column} />

    {#if selectedValues.size === 0}
        <div class="flex items-center gap-2">
            <Select.Root type="single" value={currentOperator} onValueChange={(value) => {
                if (value) updateOperator(value as TextFilterOperator);
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

            <Input
                    value={current.value}
                    placeholder="Enter filter text..."
                    oninput={(event) => updateValue(event.currentTarget.value)}
            />
        </div>
    {/if}

    {#if !(column.getIsFiltered() && current.operator !== "in")}
        <FacetedMultiSelectFilter selectedValues={selectedValues} updateValues={updateValues} options={options}/>
    {/if}
</div>