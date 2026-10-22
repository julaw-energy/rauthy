<script lang="ts" generics="TData extends RowData">
  import {ChevronFirstIcon, ChevronLastIcon, ChevronLeftIcon, ChevronRightIcon} from "@lucide/svelte";
  import {Button} from "@/components/ui/button";
  import * as Pagination from "$lib/components/ui/pagination";
  import type {PaginationState, RowData} from "@tanstack/svelte-table";
  import type { PaginationData } from "./types"
  import type {FeaturedTable} from "./data-table-features";

  type PaginationProps<TData extends RowData> = {
      table: FeaturedTable<TData>,
      paginationState: PaginationState,
      paginationData?: PaginationData
  }

  let { table, paginationState, paginationData }: PaginationProps<TData> = $props();
  let currentPage = $derived(paginationState.pageIndex + 1);
</script>

<Pagination.Root
        count={paginationData ? paginationData.totalRows : 0}
        page={currentPage}
        perPage={paginationState.pageSize}
        class="w-fit mx-0"
>
    {#snippet children({pages})}
        <Pagination.Content>
            <Pagination.Item>
                <Button
                        variant="ghost"
                        size="sm"
                        onclick={() => table.firstPage()}
                        disabled={!table.getCanPreviousPage()}
                >
                    <ChevronFirstIcon/>
                </Button>
            </Pagination.Item>
            <Pagination.Item>
                <Button
                        variant="ghost"
                        size="sm"
                        onclick={() => table.previousPage()}
                        disabled={!table.getCanPreviousPage()}
                >
                    <ChevronLeftIcon data-icon="inline-start"/>
                </Button>
            </Pagination.Item>
            {#each pages as page (page.key)}
                {#if page.type === "ellipsis"}
                    <Pagination.Item>
                        <Pagination.Ellipsis/>
                    </Pagination.Item>
                {:else}
                    <Pagination.Item>
                        <Pagination.Link
                                {page}
                                isActive={currentPage === page.value}
                                onclick={() => table.setPageIndex(page.value - 1)}
                        >
                            {page.value}
                        </Pagination.Link>
                    </Pagination.Item>
                {/if}
            {/each}
            <Pagination.Item>
                <Button
                        variant="ghost"
                        size="sm"
                        onclick={() => table.nextPage()}
                        disabled={!table.getCanNextPage()}
                >
                    <ChevronRightIcon data-icon="inline-end"/>
                </Button>
            </Pagination.Item>
            <Pagination.Item>
                <Button
                        variant="ghost"
                        size="sm"
                        onclick={() => table.lastPage()}
                        disabled={!table.getCanNextPage()}
                >
                    <ChevronLastIcon/>
                </Button>
            </Pagination.Item>
        </Pagination.Content>
    {/snippet}
</Pagination.Root>