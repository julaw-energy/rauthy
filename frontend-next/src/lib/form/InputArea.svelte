<script lang="ts">
    import { Textarea } from '$lib/components/ui/textarea';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';

    let {
        ref = $bindable(),
        id = genKey(),
        name,
        value = $bindable(''),
        label = '',

        rows = 3,
        placeholder = '',
        errMsg = '',
        disabled = false,
        fontMono = false,

        maxLength,
        required = false,
        pattern,
        isError = $bindable(false),

        width = 'inherit',

        onBlur,
        onLeft,
        onRight,
        onUp,
        onDown,
    }: {
        ref?: undefined | HTMLTextAreaElement;
        id?: string;
        name?: string;
        value?: string;
        label?: string;
        rows?: number;
        placeholder?: string;
        errMsg?: string;
        disabled?: boolean | null | undefined;
        fontMono?: boolean | null | undefined;
        maxLength?: number | null | undefined;
        step?: number;
        required?: boolean;
        pattern?: string;
        isError?: boolean;
        width?: string;

        onBlur?: () => void;
        onLeft?: () => void;
        onRight?: () => void;
        onUp?: () => void;
        onDown?: () => void;
    } = $props();

    // shadcn refs have a `null` fallback; `bind:ref` of the wrapper may start as undefined
    let inner: HTMLTextAreaElement | null = $state(null);
    $effect(() => {
        ref = inner ?? undefined;
    });

    let t = useI18n();

    function isValid() {
        let validity = ref?.validity;
        isError = validity ? !validity.valid : false;
        return !isError;
    }

    function onkeydown(ev: KeyboardEvent) {
        switch (ev.code) {
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
</script>

<div class="grid w-full gap-1.5" style:max-width={width !== 'inherit' ? width : undefined}>
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <Textarea
        bind:ref={inner}
        {id}
        {name}
        title={errMsg}
        aria-label={label || placeholder}
        bind:value
        {rows}
        {placeholder}
        {disabled}
        maxlength={maxLength || undefined}
        required={required || undefined}
        aria-invalid={isError}
        class={fontMono ? 'font-mono' : ''}
        oninput={isValid}
        oninvalid={(ev) => {
            ev.preventDefault();
            isError = true;
        }}
        onblur={() => {
            isValid();
            onBlur?.();
        }}
        {onkeydown}
    />
    {#if isError}
        <p class="text-destructive text-xs" aria-live="assertive">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
