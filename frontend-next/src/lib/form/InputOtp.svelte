<script lang="ts">
    import type { FullAutoFill } from 'svelte/elements';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { PATTERN_OTP_CODE } from '$utils/patterns';
    import Template from '$lib5/Template.svelte';
    import { TPL_OTP_LENGTH } from '$utils/constants';

    let t = useI18n();
    let otpSize = $state(8);

    let {
        ref = $bindable(),
        typ = 'text',
        id = 'otp',
        name = 'otp',
        value = $bindable(''),
        label = t.mfa.otp.code,

        autocomplete = 'one-time-code',
        errMsg,
        isError = $bindable(false),

        onBlur,
        onEnter,
        onInput,
    }: {
        ref?: undefined | HTMLInputElement;
        typ?: string;
        id?: string;
        name?: string;
        value?: string | number | null;
        label?: string;
        autocomplete?: FullAutoFill | null | undefined;
        placeholder?: string;
        errMsg?: string;
        isError?: boolean;

        onBlur?: () => void;
        onEnter?: () => void;
        onInput?: () => void;
    } = $props();

    // shadcn refs have a `null` fallback; `bind:ref` of the wrapper may start as undefined
    let inner: HTMLInputElement | null = $state(null);
    $effect(() => {
        ref = inner ?? undefined;
    });

    let placeholder = $derived('0'.repeat(otpSize));

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

<Template id={TPL_OTP_LENGTH} bind:value={otpSize} />

<div class="grid w-full gap-1.5">
    <Label for={id}>{label}<span class="text-destructive">*</span></Label>
    <Input
        bind:ref={inner}
        type={typ}
        {id}
        {name}
        inputmode="numeric"
        title={errMsg}
        aria-label={label || placeholder}
        bind:value
        {autocomplete}
        {placeholder}
        minlength={6}
        maxlength={8}
        required
        aria-invalid={isError}
        pattern={PATTERN_OTP_CODE}
        oninput={() => {
            isValid();
            onInput?.();
        }}
        oninvalid={(ev) => {
            ev.preventDefault();
            isError = true;
        }}
        onblur={() => {
            isValid();
            onBlur?.();
        }}
        onkeydown={(ev) => {
            if (ev.code === 'Enter' && isValid()) onEnter?.();
        }}
    />
    {#if isError}
        <p class="text-destructive text-xs" aria-live="assertive">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
