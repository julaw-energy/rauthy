<script lang="ts">
    import { untrack } from 'svelte';
    import type { FullAutoFill } from 'svelte/elements';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';

    let {
        ref = $bindable(),
        typ = 'text',
        id = genKey(),
        name,
        value = $bindable(),
        label = '',

        autocomplete = 'on',
        placeholder = '',
        disabled = false,

        minLength,
        maxLength,
        min,
        max,
        step = 1,

        required = false,
        pattern,
        errMsg,
        isError = $bindable(false),

        width = 'inherit',

        onBlur,
        onEnter,
        onInput,
        onLeft,
        onRight,
        onUp,
        onDown,
        onSubmit,
    }: {
        ref?: undefined | HTMLInputElement;
        typ?: string;
        id?: string;
        name?: string;
        value?: string | number | null;
        label?: string;
        autocomplete?: FullAutoFill | null | undefined;
        placeholder?: string;
        disabled?: boolean | null | undefined;
        minLength?: number | null | undefined;
        maxLength?: number | null | undefined;
        min?: string | number | null | undefined;
        max?: string | number | null | undefined;
        step?: number | string | null | undefined;
        required?: boolean;
        pattern?: string;
        errMsg?: string;
        isError?: boolean;
        width?: string;

        onBlur?: () => void;
        onEnter?: () => void;
        onInput?: () => void;
        onLeft?: () => void;
        onRight?: () => void;
        onUp?: () => void;
        onDown?: () => void;
        onSubmit?: () => void;
    } = $props();

    // shadcn refs have a `null` fallback; `bind:ref` of the wrapper may start as undefined
    let inner: HTMLInputElement | null = $state(null);
    $effect(() => {
        ref = inner ?? undefined;
    });

    let t = useI18n();
    let touched = $state(false);

    $effect(() => {
        // re-validate on every value change once the field was touched
        value;
        if (untrack(() => touched)) {
            isValid();
        }
    });

    function onblur() {
        touched = true;
        isError = false;
        requestAnimationFrame(() => {
            isValid();
            onBlur?.();
        });
    }

    function oninput() {
        touched = true;
        isValid();
        onInput?.();
    }

    function oninvalid(ev: Event) {
        ev.preventDefault();
        isError = true;
    }

    function onkeydown(ev: KeyboardEvent) {
        isValid();
        switch (ev.code) {
            case 'Enter':
                if (isValid()) onEnter?.();
                break;
            case 'ArrowUp':
                onUp?.();
                break;
            case 'ArrowDown':
                onDown?.();
                break;
            case 'ArrowLeft':
                onLeft?.();
                break;
            case 'ArrowRight':
                onRight?.();
                break;
        }
    }

    function isValid() {
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
        <Label for={id}>
            {label}{#if required}<span class="text-destructive">*</span>{/if}
        </Label>
    {/if}
    <Input
        bind:ref={inner}
        type={typ}
        {id}
        {name}
        title={errMsg}
        aria-label={label || placeholder}
        bind:value
        {autocomplete}
        {placeholder}
        {disabled}
        minlength={minLength || undefined}
        maxlength={maxLength || undefined}
        min={min || undefined}
        max={max || undefined}
        {step}
        required={required || undefined}
        aria-required={required || false}
        aria-invalid={isError}
        pattern={pattern || undefined}
        {oninput}
        {oninvalid}
        {onblur}
        {onkeydown}
        onsubmit={onSubmit}
    />
    {#if isError}
        <p class="text-destructive text-xs" aria-live="assertive">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
