<script lang="ts">
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';
    import { fmtTimeInput } from '$utils/form';

    let {
        id = genKey(),
        name = 'time',
        value = $bindable(fmtTimeInput()),
        label = '',
        ariaLabel = '',
        errMsg = '',
        disabled,
        min = '00:00',
        max = '23:59',
        required,
        width,
        onEnter,
        onLeft,
        onRight,
        onUp,
        onDown,
    }: {
        id?: string;
        name?: string;
        value?: string;
        label?: string;
        ariaLabel?: string;
        errMsg?: string;
        disabled?: boolean;
        min?: string;
        max?: string;
        required?: boolean;
        width?: string;
        openTop?: boolean;
        noEmptyLabelHeight?: boolean;

        onEnter?: () => void;
        onLeft?: () => void;
        onRight?: () => void;
        onUp?: () => void;
        onDown?: () => void;
    } = $props();

    let t = useI18n();
    let isError = $state(false);
</script>

<div class="grid w-full gap-1.5" style:max-width={width}>
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <Input
        {id}
        {name}
        type="time"
        bind:value
        {min}
        {max}
        {disabled}
        {required}
        aria-label={ariaLabel || label}
        title={errMsg}
        aria-invalid={isError}
        onkeydown={(ev) => {
            if (ev.code === 'Enter') onEnter?.();
            else if (ev.code === 'ArrowLeft') onLeft?.();
            else if (ev.code === 'ArrowRight') onRight?.();
            else if (ev.code === 'ArrowUp') onUp?.();
            else if (ev.code === 'ArrowDown') onDown?.();
        }}
        oninvalid={(ev) => {
            ev.preventDefault();
            isError = true;
        }}
        oninput={() => (isError = false)}
    />
    {#if isError}
        <p class="text-destructive text-xs">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
