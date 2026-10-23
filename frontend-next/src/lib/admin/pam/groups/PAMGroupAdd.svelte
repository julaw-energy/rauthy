<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Form from '$lib/form/Form.svelte';
    import AddFooter from '$lib5/form/AddFooter.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Input from '$lib/form/Input.svelte';
    import type {
        PamGroupCreateRequest,
        PamGroupResponse,
        PamGroupType,
        PamHostCreateRequest,
        PamHostSimpleResponse,
    } from '$api/types/pam';
    import { PATTERN_LINUX_HOSTNAME, PATTERN_LINUX_USERNAME } from '$utils/patterns';
    import Options from '$lib/Options.svelte';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { fetchPost } from '$api/fetch';

    let {
        groups,
        onCreate,
    }: {
        groups: PamGroupResponse[];
        onCreate: (group: PamGroupResponse, another?: boolean) => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();
    let err = $state('');

    const options: PamGroupType[] = ['host', 'generic', 'local'];

    let group: PamGroupCreateRequest = $state({
        name: '',
        typ: options[0],
    });

    $effect(() => {
        requestAnimationFrame(() => {
            ref?.focus();
        });
    });

    function isNameTaken() {
        let isTaken = groups.find(g => g.name === group.name);

        if (isTaken) {
            err = ta.pam.nameExistsAlready;
            return true;
        }
        return false;
    }

    let another = $state(false);

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (isNameTaken()) {
            return;
        }
        err = '';

        let res = await fetchPost<PamGroupResponse>(form.action, group);
        if (res.error) {
            err = res.error.message;
        } else if (res.body) {
            onCreate(res.body, another);
        }
    }
</script>

<div class="grid gap-4">
    <h1 class="text-lg font-semibold">{ta.pam.addGroup}</h1>

    <Form action="/auth/v1/pam/groups" {onSubmit}>
        <Input
            bind:ref
            label={ta.common.name}
            placeholder={ta.common.name}
            bind:value={group.name}
            required
            pattern={PATTERN_LINUX_USERNAME}
            onBlur={isNameTaken}
        />

        <div class="grid gap-1.5">
            <span class="text-sm font-medium">{ta.pam.groupType}</span>
            <Options ariaLabel={ta.pam.groupType} {options} bind:value={group.typ} />
        </div>

        <p class="text-muted-foreground text-sm">
            {#if group.typ === 'host'}
                {ta.pam.groupDescHost}
            {:else if group.typ === 'generic'}
                {ta.pam.groupDescGeneric}
            {:else if group.typ === 'local'}
                {ta.pam.groupDescLocal}
            {/if}
        </p>

        <AddFooter bind:another {err} />
    </Form>
</div>
