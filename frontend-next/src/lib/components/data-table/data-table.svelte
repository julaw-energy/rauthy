<script lang="ts" generics="TData extends RowData">
    // Based on julaw's data-table widget (same markup / look). Adapted for Rauthy:
    //  - `clientSide`: sorting, column filters and paging run over ALL given rows in the browser
    //  - otherwise (server mode) paging is manual and driven by `paginationData` + `handleFetch`
    //  - optional built-in `search`, `onRowClick`, `toolbar` / `rowActions` snippets, row deletion
    import {tick, untrack, type Snippet} from "svelte";
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
    import {Check, Copy, Eye, Plus, RefreshCw, Search, Trash2, Undo2} from "@lucide/svelte";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import {toast} from "svelte-sonner";
    import {Checkbox} from "$lib/components/ui/checkbox";
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
        /** Shows a "view" (eye) button per row that behaves like clicking the row. Default false: the id / name link opens the details. */
        viewButton?: boolean;
        /** Row checkboxes + "delete selected". Defaults to true when `remove` is given. */
        selectable?: boolean;
        /** Excel-like inline editing of text cells, committed or discarded all at once. */
        editable?: EditConfig<TData>;
    };

    export type EditConfig<T> = {
        /** Editable columns by column id. */
        columns: Record<string, {
            pattern?: string; required?: boolean; maxLength?: number; placeholder?: string;
            /** Text shown / edited for the cell (default: the row's field as text). */
            read?: (row: T) => string;
            /** Returns an error message when the text is not acceptable. */
            validate?: (value: string, row: T) => string | null;
            /** `text` (default), `bool` (click toggles, value is 'true' / 'false') or `select` (needs `items`). */
            kind?: 'text' | 'bool' | 'select';
            items?: {value: string; label: string}[];
        }>;
        /** Rows for which this returns false are read-only. */
        canEdit?: (row: T) => boolean;
        /** Saves the changed fields of ONE row. Throw to mark the row as failed. */
        save: (row: T, changes: Record<string, string>) => Promise<unknown>;
        /** Called once after the whole commit (e.g. to refetch). */
        onSaved?: () => void | Promise<unknown>;
    };

    const sizes = [10, 20, 50, 100, 150, 200];

    let {
        data, columns, clientSide = false, paginationData, handleFetch, searchable = false,
        searchPlaceholder = "Search…", search = $bindable(""), onFetch, fetching = false,
        onCreate, createLabel = "Create", onRowClick, remove, toolbar, rowActions, rowActionsWidth = 44,
        pageSizes = sizes, emptyText = "No results found.", initialSorting = [], initialPageSize = 20,
        getRowId, viewButton = false, selectable, editable, class: className, ...restProps
    }: DataTableProps<TData> = $props();

    let internalPagination = $state<PaginationState>({pageIndex: 0, pageSize: untrack(() => initialPageSize)});

    $effect(() => {
        if (!clientSide && paginationData) {
            // always a NEW object: the table only recomputes its page when the reference changes
            internalPagination = {pageIndex: paginationData.page - 1, pageSize: paginationData.limit};
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
    const SELECT_WIDTH = 40;
    const showView = $derived(!!onRowClick && viewButton);
    const canSelect = $derived(!!remove && (selectable ?? true));
    const delRight = 0;
    const viewRight = $derived(remove ? ACTIONS_WIDTH : 0);
    const extraRight = $derived((remove ? ACTIONS_WIDTH : 0) + (showView ? ACTIONS_WIDTH : 0));
    const actionsWidth = $derived(
        (remove ? ACTIONS_WIDTH : 0) + (showView ? ACTIONS_WIDTH : 0) + (rowActions ? rowActionsWidth : 0) + (canSelect ? SELECT_WIDTH : 0)
    );

    // ---- selection + sequential bulk delete -------------------------------------------------
    const idOf = (row: TData) => getRowId?.(row) ?? '';
    let selected = $state<Record<string, boolean>>({});
    const removable = (row: TData) => remove?.canRemove?.(row) ?? true;
    const selectedRows = $derived(data.filter((r) => selected[idOf(r)] && removable(r)));

    function togglePage(on: boolean) {
        for (const r of pageRows) selected[idOf(r)] = on;
    }

    let bulkOpen = $state(false);
    let bulkRunning = $state(false);
    let bulkDone = $state(0);
    let bulkTotal = $state(0);
    let bulkErrors = $state<{label: string; msg: string}[]>([]);
    let bulkFinished = $state(false);

    async function runBulk() {
        if (!remove) return;
        const rows = [...selectedRows];
        bulkRunning = true;
        bulkFinished = false;
        bulkDone = 0;
        bulkTotal = rows.length;
        bulkErrors = [];
        // strictly one request after the other: the API handles each delete (and its side effects) alone
        for (const row of rows) {
            try {
                await remove.run(row);
                delete selected[idOf(row)];
            } catch (e) {
                bulkErrors.push({label: remove.label?.(row) ?? idOf(row), msg: (e as {message?: string})?.message ?? String(e)});
            }
            bulkDone++;
        }
        bulkRunning = false;
        bulkFinished = true;
        await remove.onDeleted?.();
    }

    function closeBulk() {
        if (bulkRunning) return;
        bulkOpen = false;
        bulkFinished = false;
    }

    // ---- inline editing ---------------------------------------------------------------------
    let edits = $state<Record<string, Record<string, string>>>({});
    let rowErrors = $state<Record<string, string>>({});
    let committing = $state(false);
    // the one cell being typed in; every other editable cell is shown as plain text
    let editingKey = $state<string | null>(null);
    // just committed: shown in the success colour for a few seconds
    let savedMarks = $state<Record<string, string>>({});
    const SAVED_MS = 3000;
    const keyOf = (row: TData, col: string) => `${idOf(row)}\u0000${col}`;
    const focusSelect = (node: HTMLInputElement | HTMLSelectElement) => {
        node.focus();
        (node as HTMLInputElement).select?.();
    };
    const toggleBool = (row: TData, col: string) => setCell(row, col, cellValue(row, col) === 'true' ? 'false' : 'true');
    const shownValue = (row: TData, col: string) => {
        const cfg = editable?.columns[col];
        const v = cellValue(row, col);
        if (cfg?.kind === 'bool') return v === 'true' ? '✓' : '–';
        if (cfg?.kind === 'select') return cfg.items?.find((i) => i.value === v)?.label ?? v;
        return v;
    };

    const original = (row: TData, col: string) => {
        const read = editable?.columns[col]?.read;
        if (read) return read(row);
        const v = (row as Record<string, unknown>)[col];
        return v === undefined || v === null ? '' : String(v);
    };
    const editableCell = (row: TData, col: string) => !!editable?.columns[col] && (editable.canEdit?.(row) ?? true);
    const cellValue = (row: TData, col: string) => edits[idOf(row)]?.[col] ?? savedMarks[keyOf(row, col)] ?? original(row, col);
    const isDirty = (row: TData, col: string) => edits[idOf(row)]?.[col] !== undefined;

    function cellInvalid(row: TData, col: string): boolean {
        const cfg = editable?.columns[col];
        if (!cfg) return false;
        const v = cellValue(row, col);
        if (cfg.required && !v.trim()) return true;
        if (cfg.pattern && v && !new RegExp(cfg.pattern).test(v)) return true;
        if (cfg.validate && v && cfg.validate(v, row)) return true;
        return false;
    }

    function setCell(row: TData, col: string, value: string) {
        const id = idOf(row);
        if (value === original(row, col)) {
            if (edits[id]) {
                delete edits[id][col];
                if (Object.keys(edits[id]).length === 0) delete edits[id];
            }
        } else {
            (edits[id] ??= {})[col] = value;
        }
        delete rowErrors[id];
    }

    const dirtyRows = $derived(data.filter((r) => edits[idOf(r)] && Object.keys(edits[idOf(r)]).length > 0));
    const dirtyCells = $derived(dirtyRows.reduce((n, r) => n + Object.keys(edits[idOf(r)]).length, 0));
    const hasInvalid = $derived(
        dirtyRows.some((r) => Object.keys(edits[idOf(r)]).some((c) => cellInvalid(r, c)))
    );

    function discardEdits() {
        edits = {};
        rowErrors = {};
    }

    async function commitEdits() {
        if (!editable || committing || hasInvalid) return;
        committing = true;
        rowErrors = {};
        // one row after the other, like the bulk delete
        for (const row of [...dirtyRows]) {
            const id = idOf(row);
            try {
                const changes = {...edits[id]};
                await editable.save(row, changes);
                delete edits[id];
                for (const [col, value] of Object.entries(changes)) {
                    const k = keyOf(row, col);
                    savedMarks[k] = value;
                    setTimeout(() => delete savedMarks[k], SAVED_MS);
                }
            } catch (e) {
                rowErrors[id] = (e as {message?: string})?.message ?? String(e);
            }
        }
        committing = false;
        await editable.onSaved?.();
    }

    const cellSelector = () => `[data-edit-table="${tableId}"]`;

    function focusCell(rowIndex: number, col: string) {
        const el = document.querySelector<HTMLElement>(
            `${cellSelector()}[data-edit-row="${rowIndex}"][data-edit-col="${col}"]`
        );
        el?.focus();
    }

    /** Tab / Shift+Tab walk through the editable cells only (row by row), skipping buttons and links. */
    function focusNeighbour(from: HTMLElement, step: number): boolean {
        const all = Array.from(document.querySelectorAll<HTMLElement>(cellSelector()));
        const target = all[all.indexOf(from) + step];
        target?.focus();
        return !!target;
    }

    function startEdit(row: TData, col: string) {
        editingKey = keyOf(row, col);
    }

    /** Plain (not editing) cell: click / Enter / F2 start editing, arrows and Tab move between cells. */
    function cellKeydown(e: KeyboardEvent, row: TData, col: string, rowIndex: number) {
        const el = e.currentTarget as HTMLElement;
        if (e.key === 'Enter' || e.key === 'F2') {
            e.preventDefault();
            startEdit(row, col);
        } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
            e.preventDefault();
            focusCell(rowIndex + (e.key === 'ArrowDown' ? 1 : -1), col);
        } else if (e.key === 'Tab' && focusNeighbour(el, e.shiftKey ? -1 : 1)) {
            e.preventDefault();
        } else if (e.key === 'Delete' || e.key === 'Backspace') {
            e.preventDefault();
            setCell(row, col, '');
        }
    }

    /** Leaves a dropdown cell and keeps the keyboard focus in the table (step 0: the cell itself, else a neighbour). */
    async function leaveSelect(rowIndex: number, col: string, step: number, _from: HTMLElement) {
        editingKey = null;
        await tick();
        const cell = document.querySelector<HTMLElement>(
            `${cellSelector()}[data-edit-row="${rowIndex}"][data-edit-col="${col}"]`
        );
        if (!cell) return;
        if (step === 0) cell.focus();
        else if (!focusNeighbour(cell, step)) cell.focus();
    }

    /** Typing cell: Enter / Tab save the text into the cell (not to the server) and move on; Escape reverts. */
    async function editKeydown(e: KeyboardEvent, row: TData, col: string, rowIndex: number) {
        const input = e.currentTarget as HTMLInputElement;
        if (e.key === 'Escape') {
            e.preventDefault();
            setCell(row, col, original(row, col));
            editingKey = null;
            await tick();
            focusCell(rowIndex, col);
            return;
        }
        // arrow keys leave the cell like in a spreadsheet: up / down always, left / right at the text's edges
        if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
            e.preventDefault();
            editingKey = null;
            await tick();
            focusCell(rowIndex + (e.key === 'ArrowDown' ? 1 : -1), col);
            return;
        }
        if ((e.key === 'ArrowLeft' && input.selectionStart === 0 && input.selectionEnd === 0) ||
            (e.key === 'ArrowRight' && input.selectionStart === input.value.length && input.selectionEnd === input.value.length)) {
            e.preventDefault();
            editingKey = null;
            await tick();
            const cell = document.querySelector<HTMLElement>(`${cellSelector()}[data-edit-row="${rowIndex}"][data-edit-col="${col}"]`);
            if (cell) focusNeighbour(cell, e.key === 'ArrowRight' ? 1 : -1);
            return;
        }
        const step = e.key === 'Enter' ? (e.shiftKey ? -1 : 1) : e.key === 'Tab' ? (e.shiftKey ? -1 : 1) : 0;
        if (!step) return;
        e.preventDefault();
        editingKey = null;
        await tick();
        // Enter moves down the same column, Tab to the next editable cell
        if (e.key === 'Enter') {
            focusCell(rowIndex + step, col);
        } else {
            const cell = document.querySelector<HTMLElement>(
                `${cellSelector()}[data-edit-row="${rowIndex}"][data-edit-col="${col}"]`
            );
            if (cell) focusNeighbour(cell, step);
        }
    }

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
            internalPagination = {pageIndex: nextPg.pageIndex, pageSize: nextPg.pageSize};
            if (!clientSide) handleFetch?.(nextPg);
        },
        onSortingChange: (updater) => {
            sorting = typeof updater === "function" ? updater(sorting) : updater;
        },
        onColumnFiltersChange: (updater) => {
            columnFilters = typeof updater === "function" ? updater(columnFilters) : updater;
            if (clientSide) internalPagination = {...internalPagination, pageIndex: 0};
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

    const pageRows = $derived(table.getRowModel().rows.map((r) => r.original).filter(removable));
    const allOnPage = $derived(pageRows.length > 0 && pageRows.every((r) => selected[idOf(r)]));
    const someOnPage = $derived(pageRows.some((r) => selected[idOf(r)]));

    // ---- copy as CSV ------------------------------------------------------------------------
    // Drafts (uncommitted cell edits) are never copied: `data` is the saved state (the original rows
    // plus every committed change); `baseline` is the first load of this page without any changes.
    let baseline = $state<TData[] | null>(null);
    $effect(() => {
        if (!baseline && data.length > 0) baseline = $state.snapshot(data) as TData[];
    });

    const csvCell = (v: unknown) => {
        const t = v === undefined || v === null ? '' : Array.isArray(v) ? v.join('; ') : typeof v === 'object' ? JSON.stringify(v) : String(v);
        return /[",\n\r]/.test(t) ? `"${t.replace(/"/g, '""')}"` : t;
    };

    function toCsv(rows: TData[]): string {
        const cols = table.getAllLeafColumns().filter((c) => typeof c.columnDef.header === 'string' && c.accessorFn);
        const lines = [cols.map((c) => csvCell(c.columnDef.header)).join(',')];
        rows.forEach((r, i) => lines.push(cols.map((c) => csvCell(c.accessorFn!(r, i))).join(',')));
        return lines.join('\n');
    }

    async function copyCsv(original: boolean) {
        let rows: TData[];
        if (original) {
            rows = baseline ?? [];
        } else if (selectedRows.length > 0) {
            rows = selectedRows;
        } else {
            rows = table.getFilteredRowModel().rows.map((r) => r.original);
        }
        try {
            await navigator.clipboard.writeText(toCsv(rows));
            toast.success(`Copied ${rows.length} row${rows.length === 1 ? '' : 's'} as CSV${original ? ' (original data)' : ''}`);
        } catch {
            toast.error('Copying to the clipboard was blocked by the browser');
        }
    }

    // Columns share the available width in proportion to their size; the table only scrolls sideways when
    // even the minimum widths do not fit.
    const MIN_COL = 110;
    const minOf = (size: number) => Math.min(size, MIN_COL);
    const minTotal = $derived(table.getVisibleLeafColumns().reduce((n, c) => n + minOf(c.getSize()), 0));

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
        // untrack: reading the page size here must not make this effect depend on (and re-run for) itself
        if (clientSide) internalPagination = {pageIndex: 0, pageSize: untrack(() => internalPagination.pageSize)};
    });

    function rowClicked(e: MouseEvent, row: TData) {
        // editable tables: the row is a spreadsheet line - details open via the id / eye button
        if (!onRowClick || editable) return;
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
        <DropdownMenu.Root>
            <DropdownMenu.Trigger>
                {#snippet child({props})}
                    <Button {...props} size="sm" variant="outline" aria-label="Copy as CSV" title="Copy as CSV">
                        <Copy class="size-4"/>CSV
                    </Button>
                {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content align="start">
                <DropdownMenu.Item onclick={() => copyCsv(false)}>
                    {selectedRows.length > 0 ? `Selected rows (${selectedRows.length})` : 'Saved data (shown rows)'}
                </DropdownMenu.Item>
                <DropdownMenu.Item onclick={() => copyCsv(true)}>Original data (as first loaded)</DropdownMenu.Item>
            </DropdownMenu.Content>
        </DropdownMenu.Root>
        {#if onFetch}
            <Button size="sm" variant="outline" disabled={fetching} onclick={onFetch} aria-label="Refresh">
                <RefreshCw class="size-4 {fetching ? 'animate-spin' : ''}"/>
            </Button>
        {/if}
        {@render toolbar?.()}
        {#if editable && dirtyCells > 0}
            <div class="flex items-center gap-2 rounded-md border border-amber-500/60 bg-amber-500/10 px-2 py-1 text-sm">
                <span>{dirtyCells} changed in {dirtyRows.length} row{dirtyRows.length === 1 ? '' : 's'}</span>
                <Button size="sm" disabled={committing || hasInvalid} onclick={commitEdits}>
                    <Check class="size-4"/>{committing ? 'Saving...' : 'Commit'}
                </Button>
                <Button size="sm" variant="outline" disabled={committing} onclick={discardEdits}>
                    <Undo2 class="size-4"/>Discard
                </Button>
            </div>
        {/if}
        {#if canSelect && selectedRows.length > 0}
            <div class="flex items-center gap-2 rounded-md border border-destructive/50 bg-destructive/10 px-2 py-1 text-sm">
                <span>{selectedRows.length} selected</span>
                <Button size="sm" variant="destructive" onclick={() => { bulkErrors = []; bulkFinished = false; bulkOpen = true; }}>
                    <Trash2 class="size-4"/>{remove?.verb ?? 'Delete'} selected
                </Button>
                <Button size="sm" variant="outline" onclick={() => (selected = {})}>Clear</Button>
            </div>
        {/if}
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
            class="dt-border flex min-h-0 flex-col overflow-hidden rounded-md border"
            role="table"
            aria-rowcount={table.getRowModel().rows.length}
            aria-colcount={table.getVisibleLeafColumns().length}
    >
        <div class="min-h-0 overflow-auto scrollbar-thin">
            <div class="relative min-w-full" style="min-width: {minTotal + actionsWidth}px;">
                <div class="sticky top-0 z-30" role="rowgroup">
                    {#each table.getHeaderGroups() as headerGroup (headerGroup.id)}
                        <div class="dt-header dt-border flex min-w-full border-b" role="row"
                             style="width: 100%;">
                            {#if canSelect}
                                <div role="columnheader" aria-label="Select" class="dt-sticky-head dt-border flex flex-none items-center justify-center border-r"
                                     style="width: {SELECT_WIDTH}px; left: 0;">
                                    <Checkbox class="bg-background! border-foreground/45! data-checked:bg-primary! data-checked:border-primary! disabled:opacity-35" checked={allOnPage} indeterminate={!allOnPage && someOnPage}
                                              aria-label="Select all on this page"
                                              onCheckedChange={(v) => togglePage(!!v)}/>
                                </div>
                            {/if}
                            {#each headerGroup.headers as header (header.id)}
                                {@const shouldTruncate = header.column.columnDef.meta?.truncate ?? true}
                                {@const headerText = typeof header.column.columnDef.header === "string" ? header.column.columnDef.header : ""}
                                <HeaderCell {header} shouldTruncate={shouldTruncate} title={headerText}/>
                            {/each}
                            {#if rowActions}
                                <div role="columnheader" aria-label="Actions"
                                     class="dt-sticky-head dt-border flex-none"
                                     style="width: {rowActionsWidth}px; right: {extraRight}px;"></div>
                            {/if}
                            {#if showView}
                                <div role="columnheader" aria-label="View" class="dt-sticky-head dt-border flex-none"
                                     style="width: {ACTIONS_WIDTH}px; right: {viewRight}px;"></div>
                            {/if}
                            {#if remove}
                                <div role="columnheader" aria-label="Delete" class="dt-sticky-head dt-border flex-none"
                                     style="width: {ACTIONS_WIDTH}px; right: {delRight}px;"></div>
                            {/if}
                        </div>
                    {/each}
                </div>

                <div role="rowgroup">
                    {#each table.getRowModel().rows as row, rowIndex (row.id)}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <div
                                role="row"
                                tabindex={onRowClick && !editable ? 0 : undefined}
                                aria-rowindex={rowIndex + 2}
                                data-state={canSelect && selected[row.id] ? "selected" : undefined}
                                class="dt-row dt-border flex min-w-full border-b {onRowClick && !editable ? 'cursor-pointer' : ''}"
                                style="width: 100%;"
                                onclick={(e) => rowClicked(e, row.original)}
                                onkeydown={(e) => { if (e.key === 'Enter' && onRowClick && !editable && e.target === e.currentTarget) onRowClick(row.original); }}
                        >
                            {#if canSelect}
                                <div role="cell" class="dt-sticky dt-border flex flex-none items-center justify-center border-r"
                                     style="width: {SELECT_WIDTH}px; left: 0;">
                                    <Checkbox class="bg-background! border-foreground/45! data-checked:bg-primary! data-checked:border-primary! disabled:opacity-35" checked={!!selected[row.id]} disabled={!removable(row.original)}
                                              aria-label="Select row"
                                              onCheckedChange={(v) => (selected[row.id] = !!v)}/>
                                </div>
                            {/if}
                            {#each row.getVisibleCells() as cell, cellIndex (cell.id)}
                                {@const value = cell.getValue()}
                                {@const shouldTruncate = cell.column.columnDef.meta?.truncate ?? true}
                                {@const tooltip = shouldTruncate &&
                                    (typeof value === "string" || typeof value === "number" ||
                                        typeof value === "bigint" || typeof value === "boolean") ? String(value) : undefined}
                                {@const colId = cell.column.id}
                                {@const editing = editableCell(row.original, colId)}
                                <div
                                        role="cell"
                                        aria-colindex={cellIndex + 1}
                                        class="dt-border min-w-0 border-r px-1 py-1
                                            {editing && (cellInvalid(row.original, colId) || (rowErrors[row.id] && isDirty(row.original, colId))) ? 'dt-cell-error' : editing && isDirty(row.original, colId) ? 'dt-cell-dirty' : editing && savedMarks[keyOf(row.original, colId)] !== undefined ? 'dt-cell-saved' : ''}"
                                        style="flex: {cell.column.getSize()} 1 0px; min-width: {minOf(cell.column.getSize())}px;"
                                        title={editing && cellInvalid(row.original, colId) ? (editable?.columns[colId]?.validate?.(cellValue(row.original, colId), row.original) ?? "Invalid value") : rowErrors[row.id] && editing && isDirty(row.original, colId) ? rowErrors[row.id] : undefined}
                                >
                                    {#if editing}
                                        {#if editable?.columns[colId]?.kind === 'bool'}
                                            <button
                                                type="button"
                                                class="dt-edit-cell w-full text-left"
                                                data-edit-table={tableId}
                                                data-edit-row={rowIndex}
                                                data-edit-col={colId}
                                                aria-pressed={cellValue(row.original, colId) === 'true'}
                                                aria-label={String(cell.column.columnDef.header ?? colId)}
                                                onclick={() => toggleBool(row.original, colId)}
                                                onkeydown={(e) => { if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); focusCell(rowIndex + (e.key === 'ArrowDown' ? 1 : -1), colId); } else if (e.key === 'Tab' && focusNeighbour(e.currentTarget as HTMLElement, e.shiftKey ? -1 : 1)) e.preventDefault(); }}
                                            >{shownValue(row.original, colId)}</button>
                                        {:else if editingKey === keyOf(row.original, colId) && editable?.columns[colId]?.kind === 'select'}
                                            <select
                                                class="dt-input"
                                                data-edit-table={tableId}
                                                data-edit-row={rowIndex}
                                                data-edit-col={colId}
                                                aria-label={String(cell.column.columnDef.header ?? colId)}
                                                value={cellValue(row.original, colId)}
                                                use:focusSelect
                                                onchange={(e) => { setCell(row.original, colId, (e.currentTarget as HTMLSelectElement).value); leaveSelect(rowIndex, colId, 0, e.currentTarget as HTMLElement); }}
                                                onkeydown={(e) => { if (e.key === 'Escape') { e.preventDefault(); leaveSelect(rowIndex, colId, 0, e.currentTarget as HTMLElement); } else if (e.key === 'Tab') { e.preventDefault(); leaveSelect(rowIndex, colId, e.shiftKey ? -1 : 1, e.currentTarget as HTMLElement); } }}
                                                onblur={() => { if (editingKey === keyOf(row.original, colId)) editingKey = null; }}
                                            >
                                                {#each editable?.columns[colId]?.items ?? [] as it (it.value)}
                                                    <option value={it.value}>{it.label}</option>
                                                {/each}
                                            </select>
                                        {:else if editingKey === keyOf(row.original, colId)}
                                            <input
                                                class="dt-input"
                                                data-edit-table={tableId}
                                                data-edit-row={rowIndex}
                                                data-edit-col={colId}
                                                value={cellValue(row.original, colId)}
                                                placeholder={editable?.columns[colId]?.placeholder}
                                                maxlength={editable?.columns[colId]?.maxLength}
                                                aria-label={String(cell.column.columnDef.header ?? colId)}
                                                use:focusSelect
                                                oninput={(e) => setCell(row.original, colId, (e.currentTarget as HTMLInputElement).value)}
                                                onkeydown={(e) => editKeydown(e, row.original, colId, rowIndex)}
                                                onblur={() => { if (editingKey === keyOf(row.original, colId)) editingKey = null; }}
                                            />
                                        {:else}
                                            <!-- svelte-ignore a11y_no_static_element_interactions -->
                                            <div
                                                class="dt-edit-cell truncate"
                                                tabindex="0"
                                                role="textbox"
                                                aria-readonly="true"
                                                aria-label={String(cell.column.columnDef.header ?? colId)}
                                                data-edit-table={tableId}
                                                data-edit-row={rowIndex}
                                                data-edit-col={colId}
                                                onclick={() => startEdit(row.original, colId)}
                                                onkeydown={(e) => cellKeydown(e, row.original, colId, rowIndex)}
                                            >
                                                {shownValue(row.original, colId) || '\u00a0'}
                                            </div>
                                        {/if}
                                    {:else if onRowClick && (cellIndex === 0 || ['id', 'name', 'user', 'email'].includes(colId))}
                                        <!-- the first column opens the details: styled like a link -->
                                        <button type="button" class="text-primary block w-full min-w-0 text-left font-medium hover:underline"
                                                onclick={() => onRowClick(row.original)} title={tooltip}>
                                            <div class="min-w-0" class:truncate={shouldTruncate}><FlexRender cell={cell}/></div>
                                        </button>
                                    {:else}
                                        <div class="min-w-0" class:truncate={shouldTruncate} title={tooltip}>
                                            <FlexRender cell={cell}/>
                                        </div>
                                    {/if}
                                </div>
                            {/each}
                            {#if rowActions}
                                <div role="cell" class="dt-sticky dt-border flex flex-none items-center justify-center"
                                     style="width: {rowActionsWidth}px; right: {extraRight}px;">
                                    {@render rowActions(row.original)}
                                </div>
                            {/if}
                            {#if showView}
                                <div role="cell" class="dt-sticky dt-border flex flex-none items-center justify-center"
                                     style="width: {ACTIONS_WIDTH}px; right: {viewRight}px;">
                                    <button type="button" class="text-muted-foreground hover:bg-accent hover:text-foreground rounded p-1"
                                            aria-label="View / edit" title="View / edit"
                                            onclick={() => onRowClick?.(row.original)}>
                                        <Eye class="size-4"/>
                                    </button>
                                </div>
                            {/if}
                            {#if remove}
                                <div role="cell" class="dt-sticky dt-border flex flex-none items-center justify-center"
                                     style="width: {ACTIONS_WIDTH}px; right: {delRight}px;">
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

{#if remove && canSelect}
    <AlertDialog.Root open={bulkOpen} onOpenChange={(open) => { if (!open) closeBulk(); }}>
        <AlertDialog.Content>
            <AlertDialog.Header>
                <AlertDialog.Title>
                    {#if bulkFinished}
                        {bulkTotal - bulkErrors.length} of {bulkTotal} done
                    {:else}
                        {remove.verb ?? "Delete"} {selectedRows.length} selected?
                    {/if}
                </AlertDialog.Title>
                <AlertDialog.Description>
                    {#if bulkRunning}
                        One after the other: {bulkDone} / {bulkTotal}
                    {:else if bulkFinished}
                        {bulkErrors.length === 0 ? "All selected records were processed." : "These could not be processed and stay selected:"}
                    {:else}
                        {remove.description ?? "This cannot be undone."} They are processed one at a time.
                    {/if}
                </AlertDialog.Description>
            </AlertDialog.Header>
            {#if !bulkRunning && !bulkFinished}
                <ul class="max-h-48 list-disc overflow-auto pl-5 text-sm">
                    {#each selectedRows as r (idOf(r))}<li>{remove.label?.(r) ?? idOf(r)}</li>{/each}
                </ul>
            {/if}
            {#if bulkErrors.length > 0}
                <div class="max-h-48 overflow-auto rounded-md border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">
                    {#each bulkErrors as e (e.label)}<div><b>{e.label}</b>: {e.msg}</div>{/each}
                </div>
            {/if}
            <AlertDialog.Footer>
                {#if bulkFinished}
                    <Button onclick={closeBulk}>Close</Button>
                {:else}
                    <Button variant="outline" disabled={bulkRunning} onclick={closeBulk}>Cancel</Button>
                    <Button variant="destructive" disabled={bulkRunning} onclick={runBulk}>
                        <Trash2 class="size-4"/>
                        {bulkRunning ? "Working..." : (remove.verb ?? "Delete")}
                    </Button>
                {/if}
            </AlertDialog.Footer>
        </AlertDialog.Content>
    </AlertDialog.Root>
{/if}
