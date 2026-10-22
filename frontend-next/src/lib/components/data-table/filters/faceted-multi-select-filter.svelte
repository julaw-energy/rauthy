<script lang="ts" module>
  export type FacetedOption = {
    value: string;
    label: string;
    count: number;
  };
</script>

<script lang="ts">
  import {CheckIcon, SearchIcon} from "@lucide/svelte/icons";
  import { SvelteSet } from 'svelte/reactivity';
  // import { Button } from "$lib/components/ui/button";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { Input } from "$lib/components/ui/input";

  let {
    selectedValues, updateValues, options
  }: {
    selectedValues: SvelteSet<string>;
    updateValues: (values: string[]) => void;
    options: FacetedOption[];
  } = $props();

  let search = $state("");

  const visibleOptions = $derived.by(() => {
    const query = search.trim().toLocaleLowerCase();

    if (!query) return options;

    return options.filter((option) =>
      option.label.toLocaleLowerCase().includes(query)
    );
  });

  const allSelected = $derived(options.length > 0 && options.every((option) => selectedValues.has(option.value)));
  let allManuallySelected = $state(true)

  const someSelected = $derived(
    selectedValues.size > 0 && !allSelected
  );

  function commit(next: Set<string>) {
    if (next.size === 0) {
      updateValues([]);
      return;
    }

    updateValues([...next]);
  }

  function filterValue(value: string){
    if(allManuallySelected){
      allManuallySelected = false
      return new Set(options.map(option => option.value));
    } else if (!selectedValues.has(value) && options.length === selectedValues.size+1){
      allManuallySelected = true
    }
    return selectedValues
  }

  function toggleValue(value: string) {
    const next = filterValue(value);

    if (next.has(value)) {
      next.delete(value);
    } else {
      next.add(value);
    }

    commit(next);
  }

  function toggleAll() {
    if (allManuallySelected && !allSelected){
      allManuallySelected = false
      return
    }

    if (!allManuallySelected && !someSelected){
      allManuallySelected = true
      return
    }

    if(allSelected) {
      return updateValues([]);
    }
    commit(new Set(options.map((option) => option.value)));
  }

  function selectVisible() {
    const next = new SvelteSet(selectedValues);

    for (const option of visibleOptions) {
      next.add(option.value);
    }

    commit(next);
  }

</script>

<div>
  <div class="border-b py-2">
    <div class="relative">
      <SearchIcon
        class="
          pointer-events-none absolute left-2.5 top-1/2
          size-4 -translate-y-1/2
          text-muted-foreground
        "
      />

      <Input
        bind:value={search}
        placeholder="Search filters..."
        class="h-8 pl-8"
      />
    </div>
  </div>

  <div class="border-b p-1">
    <label
      class="
        flex w-full items-center gap-2
        rounded-sm px-2 py-1.5
        text-left text-sm
        hover:bg-accent
      "
    >
      <Checkbox checked={allSelected || (allManuallySelected && !someSelected)} indeterminate={someSelected} onCheckedChange={toggleAll}/>

      <span class="flex-1 font-medium">
        Select all
      </span>
    </label>

    {#if search && visibleOptions.length > 0}
      <button
        type="button"
        class="
          flex w-full items-center gap-2
          rounded-sm px-2 py-1.5
          text-left text-sm
          hover:bg-accent
        "
        onclick={selectVisible}
      >
        <CheckIcon class="size-4" />

        Select visible results
      </button>
    {/if}
  </div>

  <div class="max-h-64 overflow-y-auto scrollbar-thin p-1">
    {#each visibleOptions as option (option.value)}
      <label
        class="
          flex w-full items-center gap-2
          rounded-sm px-2 py-1.5
          text-left text-sm
          hover:bg-accent
        "
      >
        <Checkbox checked={selectedValues.has(option.value) || (allManuallySelected && !someSelected)} onCheckedChange={() => toggleValue(option.value)}
        />

        <span class="min-w-0 flex-1 truncate">
          {option.label}
        </span>

        <span
          class="
            shrink-0 text-xs tabular-nums
            text-muted-foreground
          "
        >
          {option.count}
        </span>
      </label>
    {:else}
      <div
        class="
          px-3 py-8 text-center
          text-sm text-muted-foreground
        "
      >
        No matching values.
      </div>
    {/each}
  </div>

  <div class="flex items-center justify-start border-t p-2">
    <span class="text-xs text-muted-foreground">
      {(selectedValues.size === 0 || allSelected)
        ? "All values"
        : `${selectedValues.size} selected`}
    </span>

<!--    <Button-->
<!--      type="button"-->
<!--      size="sm"-->
<!--      onclick={() => {-->
<!--      }}-->
<!--    >-->
<!--      Done-->
<!--    </Button>-->
  </div>
</div>