<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Form from '$lib/form/Form.svelte';
    import AddFooter from '$lib5/form/AddFooter.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Input from '$lib/form/Input.svelte';
    import type {
        PamUnlinkedEmailsResponse,
        PamUserCreateRequest,
        PamUserResponse,
    } from '$api/types/pam';
    import { PATTERN_LINUX_USERNAME } from '$utils/patterns';
    import Options from '$lib/Options.svelte';
    import { fetchGet, fetchPost } from '$api/fetch';
    import { onMount } from 'svelte';
    import LabeledValue from '$lib/LabeledValue.svelte';

    let {
        onCreate,
    }: {
        onCreate: (user: PamUserResponse, another?: boolean) => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let ref: undefined | HTMLInputElement = $state();

    let err = $state('');
    let options: string[] = $state([]);
    let user: PamUserCreateRequest = $state({
        username: '',
        email: '-',
    });

    onMount(() => {
        fetchUnlinkedEmails();
    });

    $effect(() => {
        requestAnimationFrame(() => {
            ref?.focus();
        });
    });

    async function fetchUnlinkedEmails() {
        let res = await fetchGet<PamUnlinkedEmailsResponse>('/auth/v1/pam/emails_unlinked');
        if (res.body) {
            options = res.body.emails;
        } else {
            console.error(res.error);
        }
    }

    let another = $state(false);

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (user.email === '-') {
            return;
        }
        err = '';

        let res = await fetchPost<PamUserResponse>(form.action, user);
        if (res.error) {
            err = res.error.message;
        } else if (res.body) {
            onCreate(res.body, another);
        }
    }
</script>

<div class="grid gap-4">
    <h1 class="text-lg font-semibold">{ta.pam.addUser}</h1>

    <Form action="/auth/v1/pam/users" {onSubmit}>
        <Input
            bind:ref
            label={ta.common.name}
            placeholder={ta.common.name}
            bind:value={user.username}
            required
            pattern={PATTERN_LINUX_USERNAME}
        />

        <LabeledValue label={ta.pam.userEmail}>
            <Options
                ariaLabel={ta.pam.userEmail}
                {options}
                bind:value={user.email}
                withSearch
                maxHeight="20rem"
            />
            {#snippet button()}
                <span class="text-destructive">*</span>
            {/snippet}
        </LabeledValue>

        <div class="bg-muted/40 grid gap-1 rounded-xl border p-3 text-sm">
            <b class="text-destructive">{ta.common.caution}:</b>
            <p class="text-muted-foreground">{ta.pam.usernameNewDesc}</p>
        </div>

        <AddFooter bind:another {err} />
    </Form>
</div>
