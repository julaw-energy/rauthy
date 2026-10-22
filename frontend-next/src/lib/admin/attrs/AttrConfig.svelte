<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchPut } from '$api/fetch';
    import Form from '$lib5/form/Form.svelte';
    import type {
        UserAttrConfigRequest,
        UserAttrConfigValueResponse,
    } from '$api/types/user_attrs.ts';
    import { PATTERN_ATTR, PATTERN_ATTR_DESC } from '$utils/patterns';
    import CheckIcon from '$lib/CheckIcon.svelte';
    import { slide } from 'svelte/transition';
    import { untrack } from 'svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import { ATTR_TYPES, attrFromText, attrToText, validateAttr, type AttrTyp } from '$lib5/attrTypes';

    let {
        attr,
        attrs,
        onSave,
    }: {
        attr: UserAttrConfigValueResponse;
        attrs: UserAttrConfigValueResponse[];
        onSave: () => void;
    } = $props();

    const width = '20rem';

    let refSubmit: undefined | HTMLButtonElement = $state();

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');
    let success = $state(false);

    let name = $state(untrack(() => attr.name));
    let desc = $state(untrack(() => attr.desc));
    let typ = $state<string>(untrack(() => attr.typ ?? ''));
    let regex = $state(untrack(() => attr.regex ?? ''));
    let errorMsg = $state(untrack(() => attr.error_msg ?? ''));
    let options = $state(untrack(() => (attr.options ?? []).join(', ')));
    let defaultValue = $state(untrack(() => attrToText(attr.typ, attr.default_value)));
    let userEditable = $state(untrack(() => attr.user_editable || false));

    let showMakeEditable = $state(false);

    $effect(() => {
        if (attr.name) {
            name = attr.name;
            desc = attr.desc;
            typ = attr.typ ?? '';
            regex = attr.regex ?? '';
            errorMsg = attr.error_msg ?? '';
            options = (attr.options ?? []).join(', ');
            defaultValue = attrToText(attr.typ, attr.default_value);
            userEditable = attr.user_editable || false;

            showMakeEditable = false;
        }
    });

    async function submitMakeEditable() {
        userEditable = true;
        refSubmit?.click();
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        err = '';

        if (name !== attr.name && attrs.find(a => a.name === name)) {
            err = ta.common.nameExistsAlready;
            return;
        }

        let t_ = (typ || undefined) as AttrTyp | undefined;
        let opts = options
            .split(',')
            .map(o => o.trim())
            .filter(Boolean);
        if (t_ === 'enum' && opts.length === 0) {
            err = 'Add at least one option for a list type';
            return;
        }
        if (regex) {
            try {
                new RegExp(regex);
            } catch {
                err = 'Invalid pattern';
                return;
            }
        }
        let defMsg = validateAttr({ name, typ: t_, regex, options: opts }, defaultValue);
        if (defMsg) {
            err = `${ta.attrs.defaultValue}: ${defMsg}`;
            return;
        }

        let payload: UserAttrConfigRequest = {
            name,
            desc: desc || undefined,
            default_value: attrFromText(t_, defaultValue),
            user_editable: userEditable || false,
            typ: t_,
            regex: regex || undefined,
            error_msg: errorMsg || undefined,
            options: t_ === 'enum' ? opts : undefined,
        };

        let res = await fetchPut(form.action, payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            onSave();
            setTimeout(() => {
                success = false;
            }, 2000);
        }
    }
</script>

<div class="max-w-xl">
    <Form action={`/auth/v1/users/attr/${attr.name}`} {onSubmit}>
        <Input
            bind:value={name}
            autocomplete="off"
            label={ta.attrs.name}
            placeholder={ta.attrs.name}
            required
            pattern={PATTERN_ATTR}
            {width}
        />
        <Input
            bind:value={desc}
            autocomplete="off"
            label={ta.attrs.desc}
            placeholder={ta.attrs.desc}
            pattern={PATTERN_ATTR_DESC}
            {width}
        />
        <InputSelect
            bind:value={typ}
            label="Type"
            items={ATTR_TYPES}
        />
        {#if typ === 'enum'}
            <Input
                bind:value={options}
                autocomplete="off"
                label="Options (comma separated)"
                placeholder="red, green, blue"
                {width}
            />
        {/if}
        {#if typ}
            <Input
                bind:value={regex}
                autocomplete="off"
                label="Extra pattern (regular expression, optional)"
                placeholder={'^[A-Z]{3}$'}
                {width}
            />
            <Input
                bind:value={errorMsg}
                autocomplete="off"
                label="Message when the value is invalid (optional)"
                placeholder="Default message for this type"
                maxLength={128}
                {width}
            />
        {/if}
        <Input
            bind:value={defaultValue}
            autocomplete="off"
            label={ta.attrs.defaultValue}
            placeholder={ta.attrs.defaultValue}
            {width}
        />

        <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex items-center gap-2">
                {ta.attrs.userEditable}
                <CheckIcon checked={userEditable} />
            </div>

            {#if !userEditable}
                <Button
                    ariaLabel={ta.attrs.makeEditable}
                    level={showMakeEditable ? 3 : 2}
                    onclick={() => (showMakeEditable = !showMakeEditable)}
                >
                    {ta.attrs.makeEditable}
                </Button>
            {/if}
        </div>

        {#if !userEditable && showMakeEditable}
            <div class="bg-muted/40 grid gap-2 rounded-xl border p-3 text-sm" transition:slide={{ duration: 150 }}>
                <p>{ta.attrs.makeEditableP1}</p>
                <p class="text-destructive">{@html ta.attrs.makeEditableP2}</p>
                <p class="text-destructive">{ta.attrs.makeEditableP3}</p>
                <div>
                    <Button ariaLabel={ta.attrs.makeEditable} level={-1} onclick={submitMakeEditable}>
                        {ta.attrs.makeEditable}
                    </Button>
                </div>
            </div>
        {/if}

        <div class="flex items-center gap-3">
            <Button type="submit" level={showMakeEditable ? 2 : 1} bind:ref={refSubmit}>
                {t.common.save}
            </Button>
            <Msg {success} {err} />
        </div>
    </Form>
</div>
