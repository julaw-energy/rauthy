<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchPut } from '$api/fetch';
    import Form from '$lib5/form/Form.svelte';
    import LabeledValue from '$lib5/LabeledValue.svelte';
    import type { RoleRequest, RoleResponse } from '$api/types/roles.ts';
    import { PATTERN_ROLE_SCOPE } from '$utils/patterns';
    import { untrack } from 'svelte';
    import { parseJsonValue, stringifyJsonValue } from '$utils/jsonValue';
    import InputArea from '$lib/form/InputArea.svelte';

    let {
        role,
        roles,
        onSave,
    }: {
        role: RoleResponse;
        roles: RoleResponse[];
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');
    let success = $state(false);
    let isRauthyAdmin = $derived(role.name === 'rauthy_admin');

    let name = $state(untrack(() => role.name));
    let jsonMeta = $state(untrack(() => stringifyJsonValue(role.meta) || ''));

    $effect(() => {
        if (role.id) {
            name = role.name;
            jsonMeta = stringifyJsonValue(role.meta) || '';
        }
    });

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        err = '';

        if (name !== role.name && roles.find(r => r.name === name)) {
            err = ta.common.nameExistsAlready;
            return;
        }

        let payload: RoleRequest = {
            role: name,
            meta: parseJsonValue(jsonMeta),
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
    <Form action={`/auth/v1/roles/${role.id}`} {onSubmit}>
        <LabeledValue label="ID" mono>
            {role.id}
        </LabeledValue>

        <Input
            bind:value={name}
            autocomplete="off"
            label={ta.roles.name}
            placeholder={ta.roles.name}
            disabled={isRauthyAdmin}
            required
            pattern={PATTERN_ROLE_SCOPE}
        />

        {#if isRauthyAdmin}
            <p class="text-muted-foreground text-sm">{@html ta.roles.adminNoMod}</p>
        {:else}
            <InputArea
                label={ta.common.jsonMeta}
                placeholder={ta.common.jsonMeta}
                rows={12}
                fontMono
                bind:value={jsonMeta}
            />

            <div class="flex items-center gap-3">
                <Button type="submit">
                    {t.common.save}
                </Button>
                <Msg {success} />
            </div>
        {/if}

        <Msg {err} />
    </Form>
</div>
