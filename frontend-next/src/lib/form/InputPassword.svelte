<script lang="ts">
    import type { FullAutoFill } from 'svelte/elements';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import Button from '$lib5/button/Button.svelte';
    import IconClipboard from '$icons/IconClipboard.svelte';
    import IconEye from '$icons/IconEye.svelte';
    import IconEyeSlash from '$icons/IconEyeSlash.svelte';

    let {
        ref = $bindable(),
        type = $bindable('password'),
        id,
        name = 'password',
        value = $bindable(''),
        label = 'Password',
        ariaLabel = 'Password',
        autocomplete = 'current-password',
        placeholder = 'Password',
        disabled = false,
        maxLength,
        required,
        pattern,
        errMsg,
        width = 'inherit',
        showCopy = false,

        reportValidity = $bindable(),
        onBlur,
        onEnter,
        onInput,
    }: {
        ref?: undefined | HTMLInputElement;
        type?: string;
        id?: undefined | string;
        name?: string;
        value?: string;
        label?: string;
        ariaLabel?: string;
        autocomplete?: FullAutoFill | null | undefined;
        placeholder?: string;
        disabled?: boolean | null | undefined;
        maxLength?: number | null | undefined;
        min?: string;
        max?: string;
        required?: boolean;
        pattern?: string;
        errMsg?: string;
        width?: string;
        showCopy?: boolean;

        reportValidity?: () => void;
        onBlur?: () => void;
        onEnter?: () => void;
        onInput?: () => void;
    } = $props();

    // shadcn refs have a `null` fallback; `bind:ref` of the wrapper may start as undefined
    let inner: HTMLInputElement | null = $state(null);
    $effect(() => {
        ref = inner ?? undefined;
    });

    let t = useI18n();
    let isError = $state(false);
    const inputId = $derived(id ?? `pw-${name}`);

    reportValidity = isValid;

    function copy() {
        if (navigator.clipboard) {
            navigator.clipboard.writeText(value);
        } else {
            console.error('Copy to clipboard is only available in secure contexts');
        }
    }

    function toggleView() {
        type = type === 'password' ? 'text' : 'password';
    }

    function onblur() {
        isValid();
        onBlur?.();
    }

    function oninput() {
        isValid();
        onInput?.();
    }

    function oninvalid(ev: Event) {
        ev.preventDefault();
        isError = true;
    }

    function onkeydown(ev: KeyboardEvent) {
        if (ev.code === 'Enter' && isValid()) {
            onEnter?.();
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
        <Label for={inputId}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <div class="relative">
        <Input
            bind:ref={inner}
            {type}
            id={inputId}
            {name}
            title={errMsg}
            aria-label={ariaLabel}
            class={showCopy ? 'pr-20' : 'pr-11'}
            bind:value
            {autocomplete}
            placeholder={placeholder || label}
            {disabled}
            required={required || undefined}
            aria-required={required || false}
            aria-invalid={isError}
            maxlength={maxLength || undefined}
            pattern={pattern || undefined}
            {oninput}
            {oninvalid}
            {onblur}
            {onkeydown}
        />
        <div class="absolute top-1/2 right-1 flex -translate-y-1/2 items-center">
            {#if showCopy}
                <Button ariaLabel={t.common.copyToClip} invisible onclick={copy}>
                    <span title={t.common.copyToClip}><IconClipboard width="1.1rem" /></span>
                </Button>
            {/if}
            <Button ariaLabel={type === 'password' ? t.common.show : t.common.hide} invisible onclick={toggleView}>
                <span title={type === 'password' ? t.common.show : t.common.hide}>
                    {#if type === 'password'}<IconEyeSlash width="1.1rem" />{:else}<IconEye width="1.1rem" />{/if}
                </span>
            </Button>
        </div>
    </div>
    {#if isError}
        <p class="text-destructive text-xs" aria-live="assertive">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
