<script lang="ts">
    let { row }: { row: { name: string; hrefPrefix: string } } = $props();
</script>

<a class="text-primary font-mono hover:underline" href={`${row.hrefPrefix}/${row.name}`}>{row.name}</a>
