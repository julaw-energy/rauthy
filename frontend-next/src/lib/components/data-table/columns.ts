// Small column builders so every Rauthy list page declares its columns in a few lines.
import { renderComponent, type RowData } from '@tanstack/svelte-table';
import type { Component } from 'svelte';
import { formatDateFromTs } from '$utils/helpers';
import type { FeaturedColumnDef } from './data-table-features';

type Opts<T extends RowData> = Partial<FeaturedColumnDef<T>>;

/** Free text column; gets the text funnel. */
export function textCol<T extends RowData>(key: keyof T & string, header: string, o: Opts<T> = {}): FeaturedColumnDef<T> {
    return {
        accessorKey: key,
        header,
        size: 200,
        meta: { dataType: 'text', filter: { type: 'text' } },
        ...o
    } as FeaturedColumnDef<T>;
}

/** Text computed from the row (e.g. joined names). */
export function fnTextCol<T extends RowData>(id: string, header: string, fn: (row: T) => string | undefined, o: Opts<T> = {}): FeaturedColumnDef<T> {
    return {
        id,
        header,
        size: 200,
        accessorFn: (r: T) => fn(r) ?? '',
        meta: { dataType: 'text', filter: { type: 'text' } },
        ...o
    } as FeaturedColumnDef<T>;
}

export function numCol<T extends RowData>(key: keyof T & string, header: string, o: Opts<T> = {}): FeaturedColumnDef<T> {
    return {
        accessorKey: key,
        header,
        size: 120,
        meta: { dataType: 'number', filter: { type: 'number' } },
        ...o
    } as FeaturedColumnDef<T>;
}

/** Unix timestamp (seconds by default, `ms` for milliseconds) rendered as a local date + time. */
export function dateCol<T extends RowData>(key: keyof T & string, header: string, o: Opts<T> & { ms?: boolean } = {}): FeaturedColumnDef<T> {
    const { ms, ...rest } = o;
    const mult = ms ? 1 : 1000;
    return {
        id: key,
        header,
        size: 190,
        accessorFn: (r: T) => {
            const v = (r as Record<string, unknown>)[key];
            return typeof v === 'number' && v > 0 ? v * mult : undefined;
        },
        cell: ({ getValue }: { getValue: () => unknown }) => {
            const v = getValue() as number | undefined;
            return v ? formatDateFromTs(v / 1000) : '-';
        },
        sortUndefined: 'last',
        meta: { dataType: 'date-time', filter: { type: 'date-time' } },
        ...rest
    } as FeaturedColumnDef<T>;
}

export function boolCol<T extends RowData>(key: keyof T & string, header: string, o: Opts<T> = {}): FeaturedColumnDef<T> {
    return {
        accessorKey: key,
        header,
        size: 110,
        cell: ({ getValue }: { getValue: () => unknown }) => (getValue() ? '✓' : '–'),
        meta: { dataType: 'boolean', filter: { type: 'boolean' } },
        ...o
    } as FeaturedColumnDef<T>;
}

/** Column rendered by a Svelte component that receives the row as `row` (plus `extra` props). */
export function componentCol<T extends RowData, P extends Record<string, unknown>>(
    id: string,
    header: string,
    comp: Component<P & { row: T }>,
    extra?: Omit<P, 'row'>,
    o: Opts<T> = {}
): FeaturedColumnDef<T> {
    return {
        id,
        header,
        size: 260,
        enableColumnFilter: false,
        cell: ({ row }: { row: { original: T } }) =>
            renderComponent(comp as Component<Record<string, unknown>>, { row: row.original, ...(extra ?? {}) }),
        ...o
    } as FeaturedColumnDef<T>;
}
