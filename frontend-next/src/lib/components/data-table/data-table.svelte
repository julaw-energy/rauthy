<script lang="ts" generics="TData extends RowData">
    // Based on julaw's data-table widget (same markup / look). Adapted for Rauthy:
    //  - `clientSide`: sorting, column filters and paging run over ALL given rows in the browser
    //  - otherwise (server mode) paging is manual and driven by `paginationData` + `handleFetch`
    //  - optional built-in `search`, `onRowClick`, `toolbar` / `rowActions` snippets, row deletion
    import {untrack, type Snippet} from "svelte";
    import {
        type ColumnDef,
        type RowData,
        type PaginationState,
        type SortingState,
        type ColumnFiltersState,
        FlexRender,
        type columnResizingState,
        createTable
    } from "@tanstack/svelte-table";
    import {
        features,
        type DataTableFeatures,
        type FeaturedColumnDef,
        type FeaturedFilterFn
    } from "./data-table-features";
    import Pagination from "./pagination.svelte";
    import * as Select from "$lib/components/ui/select/index.js";
    import {Label} from "@/components/ui/label";
    import {Input} from "@/components/ui/input";
    import {Button} from "@/components/ui/button";
    import {Plus, RefreshCw, Search, Trash2} from "@lucide/svelte";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import type {RowDeletion} from "./row-deletion";
    import type {FilterValue} from "@/types/tanstack-table";
    import HeaderCell from "./header-cell.svelte"
    import type {HTMLAttributes} from "svelte/elements";
    import type {PaginationData} from "./types";
    import {autoDetect} from "./filters/utils";

    const defaultColumn: Partial<FeaturedColumnDef<TData>> = {
        size: 150,
        minSize: 20,
        maxSize: Number.MAX_SAFE_INTEGER,
        filterFn: autoDetect as FeaturedFilterFn<TData>
    }

    const tableId = crypto.randomUUID();

    type DataTableProps<TData extends RowData> = Omit<HTMLAttributes<HTMLDivElement>, "children"> & {
        data: TData[];
        columns: ColumnDef<DataTableFeatures, TData>[];
        /** Sort / filter / page over all `data` in the browser (no server round trips). */
        clientSide?: boolean;
        /** Server mode only: where we are and how many rows exist in total. */
        paginationData?: PaginationData;
        /** Server mode only: called whenever page or page size change. */
        handleFetch?: (pg: PaginationState) => void;
        /** Shows a search field that filters rows by any visible text (clientSide). */
        searchable?: boolean;
        searchPlaceholder?: string;
        search?: string;
        /** Shows a Refresh button (and spinner while `fetching`). */
        onFetch?: () => void;
        fetching?: boolean;
        /** Shows a primary "add" button. */
        onCreate?: () => void;
        createLabel?: string;
        /** Called when a row is clicked (not when an inner button / link is). */
        onRowClick?: (row: TData) => void;
        /** Adds a delete button per row, with a confirmation. */
        remove?: RowDeletion<TData>;
        /** Extra content in the toolbar, left of the page size picker. */
        toolbar?: Snippet;
        /** Extra per-row cell on the right (e.g. an actions menu). */
        rowActions?: Snippet<[TData]>;
        rowActionsWidth?: number;
        pageSizes?: number[];
        emptyText?: string;
        initialSorting?: SortingState;
        initialPageSize?: number;
        getRowId?: (row: TData) => string;
    };

    const sizes = [10, 20, 50, 100, 150, 200];

    let {
        data, columns, clientSide = false, paginationData, handleFetch, searchable = false,
        searchPlaceholder = "Search…", search = $bindable(""), onFetch, fetching = false,
        onCreate, createLabel = "Create", onRowClick, remove, toolbar, rowActions, rowActionsWidth = 44,
        pageSizes = sizes, emptyText = "No results found.", initialSorting = [], initialPageSize = 20,
        getRowId, class: className, ...restProps
    }: DataTableProps<TData> = $props();

    let internalPagination = $state<PaginationState>({pageIndex: 0, pageSize: untrack(() => initialPageSize)});

    $effect(() => {
        if (!clientSide && paginationData) {
            internalPagination.pageIndex = paginationData.page - 1;
            internalPagination.pageSize = paginationData.limit;
        }
    });

    let sorting = $state<SortingState>(untrack(() => initialSorting));
    let columnFilters = $state<ColumnFiltersState>([]);
    let columnSizing = $state({});

    let columnResizing = $state<columnResizingState>({
        startOffset: null,
        startSize: null,
        deltaOffset: null,
        deltaPercentage: null,
        isResizingColumn: false,
        columnSizingStart: []
    });

    const ACTIONS_WIDTH = 44;
    const actionsWidth = $derived((remove ? ACTIONS_WIDTH : 0) + (rowActions ? rowActionsWidth : 0));

    let toDelete = $state<TData | undefined>();
    let deleting = $state(false);
    let deleteError = $state<string[]>([]);

    async function confirmDelete() {
        if (!remove || toDelete === undefined) return;
        deleting = true;
        deleteError = [];
        try {
            await remove.run(toDelete);
            toDelete = undefined;
            await remove.onDeleted?.();
        } catch (e) {
            const msg = (e as {message?: string})?.message ?? String(e);
            deleteError = [msg];
        } finally {
            deleting = false;
        }
    }

    // Quick search over every string / number value of the row (clientSide).
    const rowsShown = $derived.by(() => {
        const q = search.trim().toLowerCase();
        if (!clientSide || !q) return data;
        return data.filter((row) => JSON.stringify(Object.values(row as object)).toLowerCase().includes(q));
    });

    const table = createTable({
        features,
        get data() {
            return rowsShown;
        },
        get columns() {
            return columns;
        },
        getRowId: (row: TData, index: number) => getRowId?.(row) ?? String(index),
        defaultColumn: defaultColumn,
        get manualPagination() {
            return !clientSide;
        },
        enableColumnResizing: true,
        columnResizeMode: "onChange",
        get rowCount() {
            return clientSide ? undefined : (paginationData?.totalRows ?? 0);
        },
        onPaginationChange: (updater) => {
            const nextPg: PaginationState = typeof updater === "function" ? updater(internalPagination) : updater;
            internalPagination.pageIndex = nextPg.pageIndex;
            internalPagination.pageSize = nextPg.pageSize;
            if (!clientSide) handleFetch?.(nextPg);
        },
        onSortingChange: (updater) => {
            sorting = typeof updater === "function" ? updater(sorting) : updater;
        },
        onColumnFiltersChange: (updater) => {
            columnFilters = typeof updater === "function" ? updater(columnFilters) : updater;
            if (clientSide) internalPagination.pageIndex = 0;
        },
        onColumnSizingChange: (updater) => {
            columnSizing = typeof updater === "function" ? updater(columnSizing) : updater;
        },
        onColumnResizingChange: (updater) => {
            columnResizing = typeof updater === "function" ? updater(columnResizing) : updater;
        },
        state: {
            get columnSizing() { return columnSizing; },
            get columnResizing() { return columnResizing; },
            get pagination() { return internalPagination; },
            get sorting() { return sorting; },
            get columnFilters() { return columnFilters; },
        },
    });

    // Footer numbers: server mode uses the server's totals, client mode what survived filtering.
    const footer: PaginationData | undefined = $derived.by(() => {
        if (!clientSide) return paginationData;
        return {
            page: internalPagination.pageIndex + 1,
            limit: internalPagination.pageSize,
            totalRows: table.getFilteredRowModel().rows.length,
        };
    });

    $effect(() => {
        // searching resets to the first page
        search;
        if (clientSide) internalPagination.pageIndex = 0;
    });

    function rowClicked(e: MouseEvent, row: TData) {
        if (!onRowClick) return;
        if ((e.target as HTMLElement).closest("button, a, input, [role=checkbox], [role=menuitem]")) return;
        onRowClick(row);
    }
</script>

<div class="flex h-full w-full max-w-full min-w-0 flex-col overflow-hidden {className}" {...restProps}>
    <div class="flex shrink-0 flex-wrap items-center gap-2 py-2">
        {#if onCreate}
            <Button size="sm" onclick={onCreate}>
                <Plus class="size-4"/>
                {createLabel}
            </Button>
        {/if}
        {#if searchable}
            <div class="relative w-full min-w-40 sm:w-64">
                <Search class="text-muted-foreground absolute top-1/2 left-2.5 size-4 -translate-y-1/2"/>
                <Input class="h-8 pl-8" placeholder={searchPlaceholder} bind:value={search} aria-label={searchPlaceholder}/>
            </div>
        {/if}
        {#if onFetch}
            <Button size="sm" variant="outline" disabled={fetching} onclick={onFetch} aria-label="Refresh">
                <RefreshCw class="size-4 {fetching ? 'animate-spin' : ''}"/>
            </Button>
        {/if}
        {@render toolbar?.()}
        <div class="ml-auto flex items-center gap-2">
            <Label class="w-fit" for={tableId + "-pagesize"}>Entries</Label>
            <Select.Root type="single" name="pageSize" value={String(internalPagination.pageSize)}
                         onValueChange={(value) => {table.setPageSize(Number(value));}}>
                <Select.Trigger class="w-fit" id={tableId + "-pagesize"}>
                    {table.atoms.pagination.get().pageSize}
                </Select.Trigger>
                <Select.Content>
                    <Select.Group>
                        {#each pageSizes as size (size)}
                            <Select.Item value={size.toString()} label={size.toString()}>{size}</Select.Item>
                        {/each}
                    </Select.Group>
                </Select.Content>
            </Select.Root>
        </div>
    </div>

    <div
            class="flex min-h-0 flex-col overflow-hidden rounded-md border"
            role="table"
            aria-rowcount={table.getRowModel().rows.length}
            aria-colcount={table.getVisibleLeafColumns().length}
    >
        <div class="min-h-0 overflow-auto scrollbar-thin">
            <div class="relative w-max min-w-full">
                <div class="sticky top-0 z-30" role="rowgroup">
                    {#each table.getHeaderGroups() as headerGroup (headerGroup.id)}
                        <div class="flex min-w-full bg-olive-200 dark:bg-olive-800" role="row"
                             style="width: {table.getTotalSize() + actionsWidth}px;">
                            {#each headerGroup.headers as header (header.id)}
                                {@const shouldTruncate = header.column.columnDef.meta?.truncate ?? true}
                                {@const headerText = typeof header.column.columnDef.header === "string" ? header.column.columnDef.header : ""}
                                <HeaderCell {header} shouldTruncate={shouldTruncate} title={headerText}/>
                            {/each}
                            {#if rowActions}
                                <div role="columnheader" aria-label="Actions"
                                     class="flex-none border-b border-gray-400 bg-olive-200 dark:bg-olive-800"
                                     style="width: {rowActionsWidth}px;"></div>
                            {/if}
                            {#if remove}
                                <div role="columnheader" aria-label="Delete"
                                     class="flex-none border-b border-gray-400 bg-olive-200 dark:bg-olive-800"
                                     style="width: {ACTIONS_WIDTH}px;"></div>
                            {/if}
                        </div>
                    {/each}
                </div>

                <div role="rowgroup">
                    {#each table.getRowModel().rows as row, rowIndex (row.id)}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <div
                                role="row"
                                tabindex={onRowClick ? 0 : undefined}
                                aria-rowindex={rowIndex + 2}
                                data-state={row.getIsSelected() ? "selected" : undefined}
                                class="flex min-w-full even:bg-gray-200 data-[state=selected]:bg-muted dark:even:bg-gray-800 {onRowClick ? 'cursor-pointer hover:bg-accent/60' : ''}"
                                style="width: {table.getTotalSize() + actionsWidth}px;"
                                onclick={(e) => rowClicked(e, row.original)}
                                onkeydown={(e) => { if (e.key === 'Enter' && onRowClick) onRowClick(row.original); }}
                        >
                            {#each row.getVisibleCells() as cell, cellIndex (cell.id)}
                                {@const value = cell.getValue()}
                                {@const shouldTruncate = cell.column.columnDef.meta?.truncate ?? true}
                                {@const tooltip = shouldTruncate &&
                                    (typeof value === "string" || typeof value === "number" ||
                                        typeof value === "bigint" || typeof value === "boolean") ? String(value) : undefined}
                                <div
                                        role="cell"
                                        aria-colindex={cellIndex + 1}
                                        class="min-w-0 flex-none border-b border-r border-gray-400 px-1 py-1"
                                        style="width: {cell.column.getSize()}px;"
                                >
                                    <div class="min-w-0" class:truncate={shouldTruncate} title={tooltip}>
                                        <FlexRender cell={cell}/>
                                    </div>
                                </div>
                            {/each}
                            {#if rowActions}
                                <div role="cell" class="flex flex-none items-center justify-center border-b border-gray-400"
                                     style="width: {rowActionsWidth}px;">
                                    {@render rowActions(row.original)}
                                </div>
                            {/if}
                            {#if remove}
                                <div role="cell" class="flex flex-none items-center justify-center border-b border-gray-400"
                                     style="width: {ACTIONS_WIDTH}px;">
                                    {#if remove.canRemove?.(row.original) ?? true}
                                        <button type="button"
                                                class="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                                                aria-label="Delete row" title="Delete"
                                                onclick={() => { deleteError = []; toDelete = row.original; }}>
                                            <Trash2 class="size-4"/>
                                        </button>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <div role="row" class="text-muted-foreground flex h-24 min-w-full items-center justify-center text-center">
                            <div role="cell">{emptyText}</div>
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    </div>
    <div class="flex shrink-0 flex-wrap items-center justify-between gap-2 py-2 text-sm">
        <div>
            {#if footer && footer.totalRows > 0}
                {@const pageLimit = footer.page * footer.limit}
                {@const i = pageLimit < footer.totalRows ? pageLimit : footer.totalRows}
                <div>Entries {((footer.page - 1) * footer.limit) + 1} to {i} of {footer.totalRows}</div>
            {/if}
        </div>
        <Pagination table={table} paginationState={internalPagination} paginationData={footer}/>
    </div>
</div>

{#if remove}
    <AlertDialog.Root open={toDelete !== undefined} onOpenChange={(open) => { if (!open && !deleting) toDelete = undefined; }}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>{remove.verb ?? "Delete"} {toDelete !== undefined ? (remove.label?.(toDelete) ?? "this record") : ""}?</AlertDialog.Title>
                <AlertDialog.Description>{remove.description ?? "This cannot be undone."}</AlertDialog.Description>
            </AlertDialog.Header>
            {#if deleteError.length > 0}
                <div class="rounded-md border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">
                    {#each deleteError as line (line)}<div>{line}</div>{/each}
                </div>
            {/if}
            <AlertDialog.Footer>
                <Button variant="outline" disabled={deleting} onclick={() => (toDelete = undefined)}>Cancel</Button>
                <Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
                    <Trash2 class="size-4"/>
                    {deleting ? "Working..." : (remove.verb ?? "Delete")}
                </Button>
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>
{/if}
