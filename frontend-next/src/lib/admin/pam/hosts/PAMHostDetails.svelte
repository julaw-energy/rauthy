<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import type {
        PamGroupResponse,
        PamHostDetailsResponse,
        PamHostSimpleResponse,
        PamHostUpdateRequest,
    } from '$api/types/pam';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import LabeledValue from '$lib/LabeledValue.svelte';
    import { PATTERN_LINUX_HOSTNAME } from '$utils/patterns';
    import Input from '$lib/form/Input.svelte';
    import Options from '$lib/Options.svelte';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import InputArea from '$lib/form/InputArea.svelte';
    import InputTags from '$lib/form/InputTags.svelte';
    import Form from '$lib/form/Form.svelte';
    import Button from '$lib/button/Button.svelte';
    import { slide } from 'svelte/transition';
    import { useI18n } from '$state/i18n.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import PAMHostSecret from '$lib/admin/pam/hosts/PAMHostSecret.svelte';
    import { untrack } from 'svelte';

    let {
        hostSimple,
        groups,
        onDelete,
    }: {
        hostSimple: PamHostSimpleResponse;
        groups: PamGroupResponse[];
        onDelete: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    const widthAreas = 'min(min(100%, 40rem), calc(100dvw - .5rem))';
    const groupNames = untrack(() => groups.filter(g => g.typ === 'host').map(g => g.name));

    let success = $state(false);
    let err = $state('');
    let showDeleteConfirm = $state(false);
    let url = $derived(`/auth/v1/pam/hosts/${hostSimple.id}`);

    let host: undefined | PamHostDetailsResponse = $state();
    let groupName = $state(untrack(() => groups.find(g => g.id === host?.gid)?.name || ''));

    $effect(() => {
        fetchDetails();
    });

    async function deleteHost() {
        let res = await fetchDelete(url);
        if (res.error) {
            err = res.error.message;
        } else {
            onDelete();
        }
    }

    async function fetchDetails() {
        let res = await fetchGet<PamHostDetailsResponse>(url);
        if (res.body) {
            if (res.body.notes) {
                res.body.notes = res.body.notes.trim();
            } else {
                res.body.notes = '';
            }
            host = res.body;
            groupName = groups.find(g => g.id === host?.gid)?.name || '';
        } else {
            err = res.error?.message || 'ERROR';
        }
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        err = '';

        let gid = groups.find(g => g.name === groupName)?.id;
        if (!gid || !host) {
            console.error('gid not found');
            return;
        }

        let notes = host.notes?.trim();
        let payload: PamHostUpdateRequest = {
            hostname: host.hostname,
            gid,
            force_mfa: host.force_mfa,
            local_password_only: host.local_password_only,
            notes: notes ? notes : undefined,
            ips: host.ips,
            aliases: host.aliases,
        };

        // The only reason we `try` here, is that we don't want to maintain a huge and error-prone
        // IPv4+IPv6 regex in the UI side. The backend parses into `IpAddr` and the only reason we
        // can `catch` here is because of an invalid IP address.
        try {
            let res = await fetchPut(form.action, payload);
            if (res.error) {
                err = res.error.message;
            } else if (res.status === 422) {
                err = 'Invalid IP Address';
            } else {
                success = true;
                setTimeout(() => {
                    success = false;
                }, 3000);
            }
        } catch (e) {
            err = 'Invalid IP Address';
        }
    }
</script>

<div class="grid max-w-2xl gap-4">
    {#if host}
        <LabeledValue label="Host ID" copyToClip={host.id} mono>
            {host.id}
        </LabeledValue>

        <div>
            <PAMHostSecret hostId={host.id} />
        </div>

        <Form action={url} {onSubmit}>
            <Input
                label="Hostname"
                placeholder="Hostname"
                bind:value={host.hostname}
                required
                pattern={PATTERN_LINUX_HOSTNAME}
                maxLength={61}
            />

            <div class="grid gap-1.5">
                <span class="text-sm font-medium">{ta.pam.groupName}</span>
                <Options ariaLabel={ta.pam.groupName} options={groupNames} bind:value={groupName} />
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

            <InputTags label={ta.pam.ipAddresses} bind:values={host.ips} />
            <InputTags label={ta.pam.hostAliases} bind:values={host.aliases} pattern={PATTERN_LINUX_HOSTNAME} />
            <InputArea label={ta.pam.notes} bind:value={host.notes} />

            <div class="flex flex-wrap items-center gap-2">
                <Button type="submit">
                    {t.common.save}
                </Button>
                <Button level={-3} onclick={() => (showDeleteConfirm = !showDeleteConfirm)}>
                    {t.common.delete}
                </Button>
                <Msg {success} />
            </div>

            {#if showDeleteConfirm}
                <div class="grid gap-2 rounded-xl border p-3" transition:slide={{ duration: 150 }}>
                    <p class="text-sm">{ta.pam.deleteHost}</p>

                    <div class="flex gap-2">
                        <Button level={-1} onclick={deleteHost}>
                            {t.common.delete}
                        </Button>
                        <Button level={2} onclick={() => (showDeleteConfirm = false)}>
                            {t.common.cancel}
                        </Button>
                    </div>
                </div>
            {/if}
        </Form>
    {/if}

    <Msg {err} />
</div>
