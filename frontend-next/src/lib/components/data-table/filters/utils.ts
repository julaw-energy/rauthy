import type {FeaturedFilterFn} from "../data-table-features";
import type {
    TextFilterOperator,
    TextFilterValue,
    DateFilterValue,
    FilterValue,
    NumberFilterValue, NumberFilterOperator, BooleanFilterValue
} from "@/types/tanstack-table";
import {getLocalTimeZone} from "@internationalized/date";

export const initialTextFilterValue: TextFilterValue = {
    type: "text",
    operator: "contains",
    value: "",
    values: []
}

export const allTextFilterOperators: TextFilterOperator[] = [ "contains", "notContains", "startsWith", "endsWith", "=", "notEquals"]

export const initialNumberFilterValue: NumberFilterValue = {
    type: "number",
    operator: "=",
    value: NaN,
    values: [],
    from: NaN,
    to: NaN
}

export const allNumberFilterOperators: NumberFilterOperator[] = [ "=", "notEquals", ">", ">=", "<", "<=", "between"]

export const initialDateFilterValue: DateFilterValue = {
    type: "date",
    from: undefined,
    to: undefined,
    values: []
}

export const initialBooleanFilterValue: BooleanFilterValue = {
    type: "boolean",
    selection: "all"
}

export const textOperatorFilter: FeaturedFilterFn = (
    row,
    columnId: string,
    filterValue: TextFilterValue
) => {
    const rawValue = row.getValue(columnId);
    if (rawValue == null) return false;

    const cellValue = String(rawValue).toLocaleLowerCase();
    const searchValue = String(filterValue?.value ?? "").toLocaleLowerCase();

    if (!searchValue && filterValue?.operator !== "in") return true;

    switch (filterValue?.operator) {
        case "startsWith":
            return cellValue.startsWith(searchValue);
        case "endsWith":
            return cellValue.endsWith(searchValue);
        case "=":
            return cellValue === searchValue;
        case "notEquals":
            return cellValue !== searchValue;
        case "in":
            return filterValue.values?.includes(String(rawValue)) ?? false;
        case "notContains":
            return !cellValue.includes(searchValue);
        case "contains":
        default:
            return cellValue.includes(searchValue);
    }
};

export const numberOperatorFilter: FeaturedFilterFn = (
    row,
    columnId: string,
    filterValue: NumberFilterValue
) => {
    const rawValue = row.getValue(columnId);
    if (rawValue == null) return false;
    const cellValue = Number(rawValue);
    const searchValue = Number(filterValue.value);

    if (!searchValue && !(filterValue.operator === "in" || filterValue.operator === "between")) return true;

    switch (filterValue.operator) {
        case ">":
            return cellValue > searchValue
        case "<":
            return cellValue < searchValue
        case "<=":
            return cellValue <= searchValue
        case ">=":
            return cellValue >= searchValue
        case "notEquals":
            return cellValue !== searchValue;
        case "between":
            return filterValue.from <= cellValue && cellValue <= filterValue.to
        case "in":
            return filterValue.values.includes(cellValue);
        default:
            return cellValue === searchValue
    }
};


export const dateRangeFilter: FeaturedFilterFn = (
    row,
    columnId,
    filterValue: DateFilterValue
) => {
    const raw = row.getValue(columnId);

    if (!raw) return false;

    const cellTime = new Date(String(raw)).getTime();
    const fromTime = filterValue?.from
        ? filterValue.from.toDate(getLocalTimeZone()).getTime()
        : undefined;
    const toTime = filterValue?.to
        ? filterValue.to.toDate(getLocalTimeZone()).getTime()
        : undefined;

    if (Number.isNaN(cellTime)) return false;
    if (fromTime !== undefined && cellTime < fromTime) return false;
    return !(toTime !== undefined && cellTime > toTime);
};

export const booleanOperatorFilter: FeaturedFilterFn = (
    row,
    columnId: string,
    filterValue: BooleanFilterValue
) => {
    const rawValue = row.getValue(columnId);
    if (rawValue == null) return false;

    const cellValue = String(rawValue);
    const searchValue = String(filterValue.selection);
    return cellValue === searchValue
};

export const autoDetect: FeaturedFilterFn = (
    row,
    columnId,
    filterValue,
    addMeta
) => {
    const filter = filterValue as FilterValue;

    if (!filter || !filter.type) return true;

    switch (filter.type) {
        case "text":
            return textOperatorFilter(row, columnId, filter, addMeta);
        case "date":
        case "date-time":
            return dateRangeFilter(row, columnId, filter, addMeta);
        case "number":
            return numberOperatorFilter(row, columnId, filter, addMeta);
        case "boolean":
            return booleanOperatorFilter(row, columnId, filter, addMeta);
        default:
            return false;
    }
};

export const selectedValuesFilter: FeaturedFilterFn = (
    row,
    columnId,
    filterValue
) => {
    const selected = filterValue as string[] | undefined;

    // No selected filter means to show every row.
    if (!selected?.length) return true;

    const value = row.getValue(columnId);

    return selected.includes(String(value));
};

// Automatically remove the filter when its array becomes empty.
selectedValuesFilter.autoRemove = (value) =>
    !Array.isArray(value) || value.length === 0;