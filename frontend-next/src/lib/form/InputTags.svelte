<script lang="ts">
    import { X } from '@lucide/svelte';
    import { Badge } from '$lib/components/ui/badge';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';

    let {
        ref = $bindable(),
        typ = 'text',
        id = genKey(),
        name,
        values = $bindable([]),
        datalist,
        label = '',

        placeholder = '',
        disabled = false,

        maxLength,
        min,
        max,

        required = false,
        pattern,
        isError = $bindable(false),
        errMsg,
        validate,

        width = 'inherit',
        maxHeightList = 'inherit',

        onBlur,
        onEnter,
        onInput,
    }: {
        ref?: undefined | HTMLInputElement;
        typ?: 'text' | 'email' | 'url';
        id?: string;
        name?: string;
        values: string[];
        datalist?: string[];
        label?: string;
        placeholder?: string;
        errMsg?: string;
        // custom validation of the current input, returns an error message if it is invalid
        validate?: (value: string) => string | undefined;
        disabled?: boolean | null | undefined;
        maxLength?: number | null | undefined;
        min?: string;
        max?: string;
        required?: boolean;
        pattern?: string;
        isError?: boolean;
        width?: string;
        maxHeightList?: string;

        onBlur?: () => void;
        onEnter?: () => void;
        onInput?: (value: string) => void;
        onSubmit?: () => void;
    } = $props();

    let t = useI18n();
    const idDatalist = genKey();

    let value = $state('');
    let validateErr = $state('');
    let list = $derived(datalist && datalist.length > 0 ? idDatalist : undefined);

    function deleteValue(v: string) {
        values = values.filter((x) => x !== v);
    }

    function onkeydown(ev: KeyboardEvent) {
        if (ev.code === 'Enter') {
            ev.preventDefault();
            handleOnEnter();
            onEnter?.();
        }
    }

    function handleOnEnter() {
        if (value && isValid()) {
            values = [...values, value];
            value = '';
        }
    }

    function isValid() {
        if (validate) {
            validateErr = (value && validate(value)) || '';
            ref?.setCustomValidity(validateErr);
        }
        let validity = ref?.validity;
        if (validity) {
            isError = !validity.valid;
            return validity.valid;
        }
        isError = false;
        return true;
    }
</script>

<div class="grid w-full gap-1.5" style:max-width={width !== 'inherit' ? width : undefined}>
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
        class="border-input focus-within:ring-ring/30 focus-within:border-ring flex min-h-9 flex-wrap items-center gap-1.5 overflow-auto rounded-xl border px-2 py-1.5 focus-within:ring-3 {isError ? 'border-destructive' : ''}"
        style:max-height={maxHeightList}
        onclick={() => ref?.focus()}
    >
        {#each values as v (v)}
            <Badge variant="secondary" class="gap-1 pr-1">
                {v}
                <button type="button" class="hover:text-destructive rounded-full" aria-label="remove {v}" onclick={() => deleteValue(v)}>
                    <X class="size-3" />
                </button>
            </Badge>
        {/each}
        <input
            bind:this={ref}
            type={typ}
            {id}
            {name}
            {list}
            bind:value
            autocomplete="off"
            title={errMsg}
            aria-label={label || placeholder}
            {placeholder}
            {disabled}
            required={required && values.length < 1}
            maxlength={maxLength || undefined}
            min={min || undefined}
            max={max || undefined}
            {pattern}
            class="placeholder:text-muted-foreground min-w-24 flex-1 bg-transparent text-sm outline-none"
            {onkeydown}
            oninput={() => {
                isValid();
                onInput?.(value);
            }}
            oninvalid={(ev) => {
                ev.preventDefault();
                isError = true;
            }}
            onblur={() => {
                isValid();
                onBlur?.();
                handleOnEnter();
            }}
        />
        {#if datalist && datalist.length > 1}
            <datalist id={idDatalist}>
                {#each datalist as v (v)}
                    <option value={v}></option>
                {/each}
            </datalist>
        {/if}
    </div>
    {#if isError}
        <p class="text-destructive text-xs" aria-live="assertive">{validateErr || errMsg || t.common.invalidInput}</p>
    {/if}
</div>
