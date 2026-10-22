import type { RowData } from "@tanstack/svelte-table";
import type { DateValue} from "@internationalized/date";


type ColumnType = "text" | "date" | "date-time" | "boolean" | "number" | "enum"

export type TextFilterOperator =
    | "contains"
    | "notContains"
    | "startsWith"
    | "endsWith"
    | "="
    | "notEquals"
    | "in";

export type TextFilterMeta = {
    type: "text";
    operators?: TextFilterOperator[];
    suggestions?: boolean;
};

type TextFilterValue = {
    type: "text";
    operator: TextFilterOperator;
    value: string;
    values: string[];
};

export type NumberFilterOperator =
    | "="
    | "notEquals"
    | ">"
    | ">="
    | "<"
    | "<="
    | "between"
    | "in";

export type NumberFilterMeta = {
    type: "number";
    operators?: NumberFilterOperator[];
};

type NumberFilterValue = {
    type: "number";
    operator: NumberFilterOperator;
    value: number;
    values: number[];
    from: number;
    to: number;
};

export type SelectFilterMeta = {
    type: "select";
    options?: Array<{
        label: string;
        value: string;
    }>;
    multiple?: boolean;
};

export type DateFilterMeta = {
    type: "date" | "date-time";
};

export type DateFilterValue = {
    type: "date" | "date-time";
    from?: DateValue;
    to?: DateValue;
    values: string[];
};

export type BooleanFilterMeta = {
    type: "boolean";
    trueLabel?: string;
    falseLabel?: string;
};

export type BooleanFilterValue = {
    type: "boolean";
    selection?: "all" | "true" | "false";
};

// Compose them back into the union
export type DataTableFilterMeta =
    | TextFilterMeta
    | SelectFilterMeta
    | DateFilterMeta
    | NumberFilterMeta
    | BooleanFilterMeta;

type FilterValue =
    | TextFilterValue
    | DateFilterValue
    | NumberFilterValue
    | BooleanFilterValue;

declare module "@tanstack/svelte-table" {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    interface ColumnMeta<TData extends RowData, TValue> {
        truncate?: boolean;
        dataType?: ColumnType;
        editable?: boolean;
        align?: "left" | "center" | "right";
        filter?: DataTableFilterMeta;
        exportable?: boolean;
        searchable?: boolean;
        /** The backend can filter this column (set by columnsFromMeta from `x-search-params`). */
        serverFilter?: boolean;
    }
}
