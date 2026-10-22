<script lang="ts">
    import * as Select from "$lib/components/ui/select/index.js";
    import {now, getLocalTimeZone, type DateValue} from "@internationalized/date";

    let {
        value = $bindable()
    }: { value?: DateValue; } = $props();

    const pad = (num: number) => String(num).padStart(2, "0");

    let timeValue = $derived(value ?? now(getLocalTimeZone()))
    let hour = $derived('hour' in timeValue ? pad(timeValue.hour) : "00");
    let minute = $derived('minute' in timeValue ? pad(timeValue.minute) : "00");
    let second = $derived('second' in timeValue ? pad(timeValue.second) : "00");

    function updateTime(type: 'hour' | 'minute' | 'second', newValue: string) {
        if (!('hour' in timeValue)) return;
        const numValue = parseInt(newValue, 10);
        value = timeValue.set({ [type]: numValue });
    }
</script>

{#snippet TimeSelector(currentVal: string, maxItems: number, type: 'hour' | 'minute' | 'second')}
    <Select.Root type="single" value={currentVal} onValueChange={(val) => updateTime(type, val)}>
        <Select.Trigger showChevron={false} class="text-brand-green dark:text-primary font-bold">
            {currentVal}
        </Select.Trigger>
        <Select.Content class="max-h-[300px] min-w-0.5">
            <Select.Group>
                {#each Array(maxItems) as _, i (i)}
                    {@const val = pad(i)}
                    <Select.Item value={val} class="w-fit">
                        {val}
                    </Select.Item>
                {/each}
            </Select.Group>
        </Select.Content>
    </Select.Root>
{/snippet}

<div class="flex items-center justify-center gap-0.5">
    {@render TimeSelector(hour, 24, 'hour')}
    <span class="font-bold text-lg">:</span>

    {@render TimeSelector(minute, 60, 'minute')}
    <span class="font-bold text-lg">:</span>

    {@render TimeSelector(second, 60, 'second')}
</div>