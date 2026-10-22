<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Form from '$lib/form/Form.svelte';
    import Button from '$lib/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Input from '$lib/form/Input.svelte';
    import type {
        PamGroupResponse,
        PamHostCreateRequest,
        PamHostSimpleResponse,
    } from '$api/types/pam';
    import { PATTERN_LINUX_HOSTNAME } from '$utils/patterns';
    import Options from '$lib/Options.svelte';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { fetchPost } from '$api/fetch';
    import { slide } from 'svelte/transition';
    import { untrack } from 'svelte';

    let {
        groups,
        onCreate,
    }: {
        groups: PamGroupResponse[];
        onCreate: (host: PamHostSimpleResponse) => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();
    let err = $state('');

    const options = untrack(() => groups.filter(g => g.typ === 'host').map(g => g.name));
    let groupName = $state(options[0]);

    let host: PamHostCreateRequest = $state({
        hostname: '',
        gid: 0,
        force_mfa: true,
        local_password_only: false,
    });

    $effect(() => {
        requestAnimationFrame(() => {
            ref?.focus();
        });
    });

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        let gid = groups.find(g => g.name === groupName)?.id;
        if (!gid) {
            console.error('gid not found');
            return;
        }
        err = '';

        host.gid = gid;
        let res = await fetchPost<PamHostSimpleResponse>(form.action, host);
        if (res.error) {
            if (res.error.message.includes('UNIQUE constraint')) {
                err = ta.pam.nameExistsAlready;
            } else {
                err = res.error.message;
            }
        } else if (res.body) {
            onCreate(res.body);
        }
    }
</script>

<div class="grid gap-4">
    <h1 class="text-lg font-semibold">{ta.pam.addHost}</h1>

    <Form action="/auth/v1/pam/hosts" {onSubmit}>
        <Input
            bind:ref
            label="Hostname"
            placeholder="Hostname"
            bind:value={host.hostname}
            required
            pattern={PATTERN_LINUX_HOSTNAME}
            maxLength={61}
        />

        <div class="grid gap-1.5">
            <span class="text-sm font-medium">{ta.pam.groupName}</span>
            <Options ariaLabel={ta.pam.groupName} {options} bind:value={groupName} />
        </div>

        <InputCheckbox ariaLabel={ta.clients.forceMfa} bind:checked={host.force_mfa}>
            {ta.clients.forceMfa}
        </InputCheckbox>

        <InputCheckbox ariaLabel={ta.pam.hostLocalPwdOnly} bind:checked={host.local_password_only}>
            {ta.pam.hostLocalPwdOnly}
        </InputCheckbox>
        {#if host.local_password_only}
            <p class="text-destructive text-sm" transition:slide={{ duration: 150 }}>
                {ta.pam.hostLocalPwdOnlyInfo}
            </p>
        {/if}

        <div class="flex items-center gap-3">
            <Button type="submit">
                {t.common.save}
            </Button>
            <Msg {err} />
        </div>
    </Form>
</div>
