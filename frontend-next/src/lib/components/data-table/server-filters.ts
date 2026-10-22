import {getContext, setContext} from "svelte";
import {getLocalTimeZone} from "@internationalized/date";
import type {FilterValue} from "@/types/tanstack-table";

/**
 * Backend filters applied from the column funnels. The values are the same objects the funnels already
 * produce for in-memory filtering; `toSearchParams` turns them into the endpoint's query params.
 */
export type ServerFilters = {
    readonly applied: Readonly<Record<string, FilterValue>>;
    /** Whether the backend can express this filter (supported operator, column has a search param). */
    canApply: (columnId: string, value: FilterValue) => boolean;
    apply: (columnId: string, value: FilterValue) => void;
    clear: (columnId: string) => void;
    clearAll: () => void;
};

const KEY = Symbol("data-table-server-filters");
export const setServerFilters = (filters: ServerFilters | undefined) => setContext(KEY, filters);
export const getServerFilters = () => getContext<ServerFilters | undefined>(KEY);

const toDate = (v: {toDate: (tz: string) => Date}) => v.toDate(getLocalTimeZone());

/** Short human description for the chip bar, e.g. `contains "uga"`. */
export function describeFilter(filter: FilterValue): string {
    switch (filter.type) {
        case "text":
            return `${filter.operator} "${filter.value}"`;
        case "number":
            return `${filter.operator} ${filter.value}`;
        case "date":
        case "date-time": {
            const fmt = (v: {toDate: (tz: string) => Date}) => v.toDate(getLocalTimeZone()).toLocaleDateString();
            return `${filter.from ? fmt(filter.from) : "…"} – ${filter.to ? fmt(filter.to) : "…"}`;
        }
        case "boolean":
            return filter.selection ?? "all";
    }
}
