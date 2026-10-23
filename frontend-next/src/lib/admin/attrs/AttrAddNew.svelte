<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import AddFooter from '$lib5/form/AddFooter.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import Switch from '$lib5/Switch.svelte';
    import { ATTR_TYPES, attrFromText, validateAttr, type AttrTyp } from '$lib5/attrTypes';
    import { PATTERN_ATTR, PATTERN_ATTR_DESC } from '$utils/patterns';
    import Form from '$lib5/form/Form.svelte';
    import { fetchPost } from '$api/fetch';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import type {
        UserAttrConfigEntity,
        UserAttrConfigRequest,
        UserAttrConfigValueResponse,
    } from '$api/types/user_attrs.ts';

    let {
        attrs,
        onSave,
    }: {
        attrs: UserAttrConfigValueResponse[];
        onSave: (name: string, another?: boolean) => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();

    let err = $state('');
    let name = $state('');
    let desc = $state('');
    let typ = $state('');
    let options = $state('');
    let regex = $state('');
    let errorMsg = $state('');
    let defaultValue = $state('');
    let userEditable = $state(false);

    $effect(() => {
        requestAnimationFrame(() => {
            ref?.focus();
        });
    });

    let another = $state(false);

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (attrs.find(a => a.name === name)) {
            err = ta.common.nameExistsAlready;
            return;
        }
        err = '';

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
            typ: t_,
            regex: regex || undefined,
            error_msg: errorMsg || undefined,
            options: t_ === 'enum' ? opts : undefined,
            default_value: attrFromText(t_, defaultValue),
            user_editable: userEditable || undefined,
        };
        let res = await fetchPost<UserAttrConfigEntity>(form.action, payload);
        if (res.body) {
            onSave(res.body.name, another);
        } else {
            err = res.error?.message || 'Error';
        }
    }
</script>

<div class="grid gap-4">
    <Form action="/auth/v1/users/attr" {onSubmit}>
        <Input
            bind:ref
            bind:value={name}
            autocomplete="off"
            label={ta.attrs.name}
            placeholder={ta.attrs.name}
            required
            pattern={PATTERN_ATTR}
        />
        <Input
            bind:value={desc}
            autocomplete="off"
            label={ta.attrs.desc}
            placeholder={ta.attrs.desc}
            pattern={PATTERN_ATTR_DESC}
        />
        <InputSelect bind:value={typ} label="Type" items={ATTR_TYPES} />
        {#if typ === 'enum'}
            <Input
                bind:value={options}
                autocomplete="off"
                label="Options (comma separated)"
                placeholder="red, green, blue"
            />
        {/if}
        {#if typ}
            <Input
                bind:value={regex}
                autocomplete="off"
                label="Extra pattern (regular expression, optional)"
                placeholder={'^[A-Z]{3}$'}
            />
            <Input
                bind:value={errorMsg}
                autocomplete="off"
                label="Message when the value is invalid (optional)"
                placeholder="Default message for this type"
                maxLength={128}
            />
        {/if}
        <Input
            bind:value={defaultValue}
            autocomplete="off"
            label={ta.attrs.defaultValue}
            placeholder={ta.attrs.defaultValue}
        />
        <Switch bind:checked={userEditable} ariaLabel={ta.attrs.userEditable}>
            {ta.attrs.userEditable}
        </Switch>

        <AddFooter bind:another {err} />
    </Form>
</div>
