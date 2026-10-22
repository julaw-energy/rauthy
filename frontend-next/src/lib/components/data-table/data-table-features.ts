import {
    columnFilteringFeature,
    columnVisibilityFeature,
    createFilteredRowModel,
    createPaginatedRowModel,
    createSortedRowModel,
    filterFn_includesString,
    rowPaginationFeature,
    rowSelectionFeature,
    rowSortingFeature,
    coreRowModelsFeature,
    columnFacetingFeature,
    columnSizingFeature,
    columnResizingFeature,
    createFacetedMinMaxValues,
    createFacetedRowModel,
    createFacetedUniqueValues,
    createCoreRowModel,
    sortFn_alphanumeric,
    sortFn_text,
    tableFeatures,
    type Header,
    type Column,
    type ColumnDef,
    type Table,
    type RowData, type CellData, type FilterFn, type Row, type TableFeatures, type ExtractFilterMeta
} from "@tanstack/svelte-table";


export const features = tableFeatures({
    columnFilteringFeature,
    columnVisibilityFeature,
    columnFacetingFeature,
    columnSizingFeature,
    columnResizingFeature,
    rowPaginationFeature,
    rowSelectionFeature,
    rowSortingFeature,
    coreRowModelsFeature,

    coreRowModel: createCoreRowModel(),
    filteredRowModel: createFilteredRowModel(),
    paginatedRowModel: createPaginatedRowModel(),
    sortedRowModel: createSortedRowModel(),
    filterFns: { includesString: filterFn_includesString },
    sortFns: { alphanumeric: sortFn_alphanumeric, text: sortFn_text },
    facetedRowModel: createFacetedRowModel(),
    facetedUniqueValues: createFacetedUniqueValues(),
    facetedMinMaxValues: createFacetedMinMaxValues(),
});

// Pass this as the first generic argument to `ColumnDef`, `Column`, `Table`,
// and `Row` so each type knows which feature APIs are available.
export type DataTableFeatures = typeof features;

export type FeaturedHeader<
    TData extends RowData = RowData,
    TValue extends CellData = CellData,
> = Header<DataTableFeatures, TData, TValue>;

export type FeaturedRow<
    TData extends RowData = RowData
> = Row<DataTableFeatures, TData>;

export type FeaturedColumn<
    TData extends RowData = RowData,
    TValue extends CellData = CellData
> = Column<DataTableFeatures, TData, TValue>;

export type FeaturedColumnDef<
    TData extends RowData = RowData,
    TValue extends CellData = CellData
> = ColumnDef<DataTableFeatures, TData, TValue>;

export type FeaturedTable<
    TData extends RowData = RowData
> = Table<DataTableFeatures, TData>;

export type FeaturedFilterFn<
    TData extends RowData = RowData
> = FilterFn<DataTableFeatures, TData>;
