<script lang="ts" generics="TData extends RowData">
  import {
    ArrowDown,
    ArrowUp,
    ChevronsUpDown
  } from "@lucide/svelte/icons";

  import type {FeaturedColumn} from "./data-table-features";
  import type {RowData} from "@tanstack/svelte-table";

  let {
    column,
    title
  }: {
    column: FeaturedColumn<TData>;
    title: string;
  } = $props();

  const sortDirection = $derived(column.getIsSorted());
  const sortIndex = $derived(column.getSortIndex());
  const showSortIndex = $derived(sortDirection !== false && sortIndex >= 0);

  function handleSort() {
    if (sortDirection === false) {
      column.toggleSorting(false, true);
      return;
    }

    if (sortDirection === "asc") {
      column.toggleSorting(true, true);
      return;
    }
    column.clearSorting();
  }
</script>

<button
        type="button"
        class="relative {showSortIndex?'size-7':''} items-center flex gap-1 rounded-sm text-left hover:bg-black/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring dark:hover:bg-white/10"
        aria-label={
          sortDirection === false ? `Sort ${title} ascending` : sortDirection === "asc" ? `Sort ${title} descending`
          : `Clear sorting for ${title}`
          }
        onclick={handleSort}
>
    <span class="flex items-center">
      {#if sortDirection === "asc"}
        <ArrowUp class="size-4" aria-hidden="true" />
      {:else if sortDirection === "desc"}
        <ArrowDown class="size-4" aria-hidden="true" />
      {:else}
        <ChevronsUpDown class="size-4 opacity-40 group-hover:opacity-80" aria-hidden="true"/>
      {/if}

        {#if showSortIndex}
          <span
                  class="
              absolute right-0.5 top-0.5
              flex size-3.5 items-center justify-center
              rounded-full bg-primary
              text-[9px] font-semibold text-primary-foreground
            "
                  aria-label={`Sort priority ${sortIndex + 1}`}
          >
            {sortIndex + 1}
          </span>
      {/if}
    </span>
</button>