<script lang="ts">
    import Input from '$lib5/form/Input.svelte';
    import InputArea from '$lib5/form/InputArea.svelte';
    import InputPhone from '$lib5/form/InputPhone.svelte';
    import InputDate from '$lib5/form/InputDate.svelte';
    import InputCountry from '$lib5/form/InputCountry.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import TZSelect from '$lib5/TZSelect.svelte';
    import Switch from '$lib5/Switch.svelte';
    import { validateAttr, attrToText, type AttrDef } from '$lib5/attrTypes';

    // One custom user attribute. The widget follows `def.typ`; `value` is always the text form
    // (see `attrFromText` for the way back). Untyped attributes keep the plain JSON text input.
    let {
        def,
        value = $bindable(''),
        label,
        onEnter,
    }: {
        def: AttrDef;
        value?: string;
        label?: string;
        onEnter?: () => void;
    } = $props();

    let touched = $state(false);
    let msg = $derived(touched ? validateAttr(def, value) : null);
    let lbl = $derived(label ?? def.name);
    let placeholder = $derived(attrToText(def.typ, def.default_value) || def.desc || '');
</script>

<div class="grid gap-1" onfocusout={() => (touched = true)}>
    {#if def.typ === 'boolean'}
        <!-- stays empty (no value) until it is toggled -->
        <Switch bind:checked={() => value === 'true', v => (value = v ? 'true' : 'false')} ariaLabel={lbl}>
            {lbl}
        </Switch>
    {:else if def.typ === 'text'}
        <InputArea bind:value label={lbl} {placeholder} />
    {:else if def.typ === 'phone'}
        <InputPhone bind:value label={lbl} {placeholder} {onEnter} />
    {:else if def.typ === 'date'}
        <InputDate bind:value label={lbl} min="0001-01-01" max="9999-12-31" {onEnter} />
    {:else if def.typ === 'country'}
        <InputCountry bind:value label={lbl} />
    {:else if def.typ === 'timezone'}
        <TZSelect bind:value />
    {:else if def.typ === 'enum'}
        <InputSelect bind:value label={lbl} items={(def.options ?? []).map(o => ({ value: o, label: o }))} />
    {:else}
        <!-- a number input binds a NUMBER; the form logic works with text, so convert in both directions -->
        <Input
            bind:value={() => value, v => (value = v === null || v === undefined ? '' : String(v))}
            autocomplete="off"
            label={lbl}
            {placeholder}
            typ={def.typ === 'email'
                ? 'email'
                : def.typ === 'url'
                  ? 'url'
                  : def.typ === 'number' || def.typ === 'integer'
                    ? 'number'
                    : 'text'}
            step={def.typ === 'integer' ? 1 : def.typ === 'number' ? 'any' : undefined}
            {onEnter}
        />
    {/if}
    {#if msg}
        <p class="text-destructive text-xs">{msg}</p>
    {/if}
</div>
