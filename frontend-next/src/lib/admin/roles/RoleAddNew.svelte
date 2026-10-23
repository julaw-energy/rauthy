<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import AddFooter from '$lib5/form/AddFooter.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import { PATTERN_ROLE_SCOPE } from '$utils/patterns';
    import Form from '$lib5/form/Form.svelte';
    import { fetchPost } from '$api/fetch';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import type { RoleRequest, RoleResponse } from '$api/types/roles.ts';
    import InputArea from '$lib/form/InputArea.svelte';
    import { parseJsonValue } from '$utils/jsonValue';

    let {
        roles,
        onSave,
    }: {
        roles: RoleResponse[];
        onSave: (id: string, another?: boolean) => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();

    let err = $state('');
    let name = $state('');
    let jsonMeta = $state('');

    $effect(() => {
        requestAnimationFrame(() => {
            ref?.focus();
        });
    });

    let another = $state(false);

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (roles.find(r => r.name === name)) {
            err = ta.common.nameExistsAlready;
            return;
        }
        err = '';

        let payload: RoleRequest = {
            role: name,
            meta: parseJsonValue(jsonMeta),
        };
        let res = await fetchPost<RoleResponse>(form.action, payload);
        if (res.body) {
            onSave(res.body.id, another);
        } else {
            err = res.error?.message || 'Error';
        }
    }
</script>

<div class="grid gap-4">
    <Form action="/auth/v1/roles" {onSubmit}>
        <Input
            bind:ref
            bind:value={name}
            autocomplete="off"
            label={ta.roles.name}
            placeholder={ta.roles.name}
            required
            pattern={PATTERN_ROLE_SCOPE}
        />
        <InputArea
            label={ta.common.jsonMeta}
            placeholder={ta.common.jsonMeta}
            bind:value={jsonMeta}
            rows={12}
            fontMono
        />

        <AddFooter bind:another {err} />
    </Form>
</div>
