<script lang="ts">
    import { PATTERN_CSS_VALUE_LOOSE } from '$utils/patterns';
    import Input from '$lib5/form/Input.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';

    let {
        label,
        value = $bindable(),
    }: {
        label: string;
        value: string;
    } = $props();

    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();
</script>

<div class="flex items-end gap-2" style:--color={value}>
    <div class="flex-1">
        <Input
            {label}
            placeholder={label}
            errMsg={ta.validation.css}
            bind:value
            required
            pattern={PATTERN_CSS_VALUE_LOOSE}
        />
    </div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
        role="none"
        class="border-input relative size-9 shrink-0 cursor-pointer overflow-hidden rounded-xl border"
        style:background={value}
        onclick={() => ref?.click()}
    >
        <input bind:this={ref} type="color" bind:value class="absolute inset-0 size-full cursor-pointer opacity-0" />
    </div>
</div>
