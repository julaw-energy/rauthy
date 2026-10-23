<script lang="ts">
    import { CalendarDays } from '@lucide/svelte';
    import * as Select from '$lib/components/ui/select/index.js';
    import { DATE_FORMATS, dateFormat, formatIsoDate, type DateFormat } from '$lib5/dateFormat.svelte';

    // today's date in local time, so the options always show what the formats look like right now
    const t = new Date();
    const sample = `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')}`;
</script>

<Select.Root type="single" value={dateFormat.format} onValueChange={(v) => dateFormat.set(v as DateFormat)}>
    <Select.Trigger class="w-auto min-w-36 gap-2" aria-label="Date format" title="Date format">
        <CalendarDays class="text-muted-foreground size-4" />
        {formatIsoDate(sample)}
    </Select.Trigger>
    <Select.Content>
        {#each DATE_FORMATS as f (f)}
            <Select.Item value={f} label={formatIsoDate(sample, f)}>{formatIsoDate(sample, f)}</Select.Item>
        {/each}
    </Select.Content>
</Select.Root>
