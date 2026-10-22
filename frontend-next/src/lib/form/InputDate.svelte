<script lang="ts">
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';
    import { fmtDateInput } from '$utils/form';
    import type { PropsInputDate } from './props';
    import { CalendarDays } from '@lucide/svelte';
    import { dateFormat, formatIsoDate } from '$lib5/dateFormat.svelte';

    // native date input: responsive, accessible, and fully compatible with <form> validation
    let {
        id = genKey(),
        name,
        value = $bindable(fmtDateInput()),
        label = '',
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
    }: PropsInputDate = $props();

    let t = useI18n();
    let isError = $state(false);
    let native: HTMLInputElement | undefined = $state();
    let shown = $derived(formatIsoDate(value));

    function openPicker() {
        try {
            native?.showPicker();
        } catch {
            native?.focus();
        }
    }

    function onkeydown(ev: KeyboardEvent) {
        switch (ev.code) {
            case 'Enter':
                onEnter?.();
                break;
            case 'ArrowLeft':
                onLeft?.();
                break;
            case 'ArrowRight':
                onRight?.();
                break;
            case 'ArrowUp':
                onUp?.();
                break;
            case 'ArrowDown':
                onDown?.();
                break;
        }
    }
</script>

<div class="grid w-full gap-1.5">
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <div class="relative">
        <button
            type="button"
            class="border-input bg-background hover:bg-accent flex h-9 w-full items-center justify-between rounded-md border px-3 text-sm shadow-xs disabled:opacity-50 border-destructive/0"
            {disabled}
            title={errMsg}
            onclick={openPicker}
            {onkeydown}
        >
            <span class={shown ? '' : 'text-muted-foreground'}>{shown || dateFormat.format}</span>
            <CalendarDays class="text-muted-foreground size-4" />
        </button>
        <!-- real input: keeps ISO value, validation and the native calendar popup -->
        <input
            bind:this={native}
            {id}
            {name}
            type="date"
            bind:value
            {min}
            {max}
            {disabled}
            {required}
            tabindex="-1"
            aria-hidden="true"
            class="pointer-events-none absolute inset-0 h-full w-full opacity-0"
            oninvalid={(ev) => {
                ev.preventDefault();
                isError = true;
            }}
            oninput={() => (isError = false)}
        />
    </div>
    {#if isError}
        <p class="text-destructive text-xs">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
