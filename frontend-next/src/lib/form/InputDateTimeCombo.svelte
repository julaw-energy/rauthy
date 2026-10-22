<script lang="ts">
    import { X } from '@lucide/svelte';
    import { fmtDateInput, fmtTimeInput } from '$utils/form';
    import Button from '$lib5/button/Button.svelte';
    import InputDate from './InputDate.svelte';
    import InputTime from './InputTime.svelte';
    import type { PropsInputDateCombo } from './props';

    let {
        id,
        name,
        value = $bindable(fmtDateInput()),
        label = '',
        labelTime = '',
        errMsg = '',
        disabled,
        min = '1900-01-01',
        max = '2100-01-01',
        required,
        onEnter,
        onLeft,
        onRight,
        onUp,
        onDown,

        timeName,
        timeValue = $bindable(fmtTimeInput()),
        timeErrMsg,
        timeMin,
        timeMax,

        withDelete,
        withTime,
        openTop = false,
    }: PropsInputDateCombo = $props();

    function reset() {
        value = '';
        timeValue = '';
    }
</script>

<div class="flex flex-wrap items-end gap-2">
    <div class="min-w-40 flex-1">
        <InputDate {id} {name} bind:value {label} {errMsg} {disabled} {min} {max} {required} {openTop} {onEnter} {onLeft} {onRight} {onUp} {onDown} />
    </div>
    {#if withTime}
        <div class="min-w-32 flex-1">
            <InputTime name={timeName} bind:value={timeValue} label={labelTime} errMsg={timeErrMsg} {disabled} min={timeMin} max={timeMax} {required} />
        </div>
    {/if}
    {#if withDelete}
        <Button invisible level={-3} onclick={reset} ariaLabel="reset"><X class="size-4" /></Button>
    {/if}
</div>
