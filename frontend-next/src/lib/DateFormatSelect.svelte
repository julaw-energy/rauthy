<script lang="ts">
    import { CalendarDays } from '@lucide/svelte';
    import HeaderPicker from '$lib/HeaderPicker.svelte';
    import { DATE_FORMATS, dateFormat, formatIsoDate, type DateFormat } from '$lib5/dateFormat.svelte';

    // today's date in local time, so the options always show what the formats look like right now
    const t = new Date();
    const sample = `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')}`;
</script>

<HeaderPicker
    icon={CalendarDays}
    ariaLabel="Date format"
    options={DATE_FORMATS.map((f) => ({ value: f, label: formatIsoDate(sample, f) }))}
    value={dateFormat.format}
    onChange={(v) => dateFormat.set(v as DateFormat)}
    label={formatIsoDate(sample)}
    class="min-w-36"
/>
