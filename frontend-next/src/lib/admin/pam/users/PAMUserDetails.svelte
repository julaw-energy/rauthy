<script lang="ts">
    import Msg from '$lib5/Msg.svelte'
    ;import * as Table from '$lib/components/ui/table';
    import type {
        PamGroupUserLink,
        PamUserDetailsResponse,
        PamUserResponse,
        PamUserUpdateRequest,
    } from '$api/types/pam';
    import LabeledValue from '$lib/LabeledValue.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Button from '$lib/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import Input from '$lib/form/Input.svelte';
    import Form from '$lib/form/Form.svelte';
    import type { PamGroupsSorted } from '$lib/admin/pam/types';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import PamAuthorizedKeys from '$lib/PamAuthorizedKeys.svelte';

    let {
        user = $bindable(),
        groupsSorted,
        onDelete,
    }: {
        user: PamUserResponse;
        groupsSorted: PamGroupsSorted;
        onDelete: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');
    let success = $state(false);
    let deleteConfirm = $state(false);
    let details: undefined | PamUserDetailsResponse = $state();

    interface LinkedGroup {
        gid: number;
        name: string;
        isMember: boolean;
        isWheel: boolean;
    }

    let groupsGeneric: LinkedGroup[] = $state([]);
    let groupsHost: LinkedGroup[] = $state([]);
    let groupsLocal: LinkedGroup[] = $state([]);
    let groupsUser: LinkedGroup[] = $state([]);

    let url = $derived(`/auth/v1/pam/users/${user.id}`);

    $effect(() => {
        fetchDetails();
    });

    async function fetchDetails() {
        deleteConfirm = false;

        let res = await fetchGet<PamUserDetailsResponse>(url);
        if (res.body) {
            details = res.body;
            buildAccess(res.body.groups);
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function buildAccess(links: PamGroupUserLink[]) {
        let generic: LinkedGroup[] = [];
        let host: LinkedGroup[] = [];
        let local: LinkedGroup[] = [];
        let user: LinkedGroup[] = [];

        for (let group of groupsSorted.generic) {
            let link = links.find(l => l.gid === group.id);
            generic.push({
                gid: group.id,
                name: group.name,
                isMember: !!link,
                isWheel: link?.wheel || false,
            });
        }

        for (let group of groupsSorted.host) {
            let link = links.find(l => l.gid === group.id);
            host.push({
                gid: group.id,
                name: group.name,
                isMember: !!link,
                isWheel: link?.wheel || false,
            });
        }

        for (let group of groupsSorted.local) {
            let link = links.find(l => l.gid === group.id);
            local.push({
                gid: group.id,
                name: group.name,
                isMember: !!link,
                isWheel: link?.wheel || false,
            });
        }

        for (let group of groupsSorted.user) {
            let link = links.find(l => l.gid === group.id);
            user.push({
                gid: group.id,
                name: group.name,
                isMember: !!link,
                isWheel: link?.wheel || false,
            });
        }

        groupsGeneric = generic;
        groupsHost = host;
        groupsLocal = local;
        groupsUser = user;
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        let links: PamGroupUserLink[] = [];

        for (let group of groupsHost) {
            if (group.isMember) {
                links.push({
                    uid: user.id,
                    gid: group.gid,
                    wheel: group.isWheel,
                });
            }
        }
        for (let group of groupsGeneric) {
            if (group.isMember) {
                links.push({
                    uid: user.id,
                    gid: group.gid,
                    wheel: group.isWheel,
                });
            }
        }
        for (let group of groupsLocal) {
            if (group.isMember) {
                links.push({
                    uid: user.id,
                    gid: group.gid,
                    wheel: group.isWheel,
                });
            }
        }
        for (let group of groupsUser) {
            if (group.isMember) {
                links.push({
                    uid: user.id,
                    gid: group.gid,
                    wheel: group.isWheel,
                });
            }
        }

        let payload: PamUserUpdateRequest = {
            shell: user.shell,
            home_dir: user.home_dir,
            groups: links,
        };
        let res = await fetchPut(url, payload);
        if (res.status === 200) {
            success = true;
            setTimeout(() => {
                success = false;
            }, 3000);
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function deleteUser() {
        let res = await fetchDelete(url);
        if (res.status === 200) {
            onDelete();
        } else {
            err = res.error?.message || 'Error';
        }
    }
</script>

{#snippet snipGroups(header: string, groups: LinkedGroup[], withWheel: boolean = false)}
    {#if groups.length > 0}
        <div class="grid gap-2">
            <h3 class="font-semibold">{header}</h3>
            <div class="rounded-xl border">
                <Table.Root>
                    <Table.Header>
                        <Table.Row>
                            <Table.Head>{ta.pam.groupName}</Table.Head>
                            <Table.Head class="text-center">{ta.pam.member}</Table.Head>
                            {#if withWheel}
                                <Table.Head class="text-center">Wheel</Table.Head>
                            {/if}
                        </Table.Row>
                    </Table.Header>
                    <Table.Body>
                        {#each groups as group (group.name)}
                            <Table.Row>
                                <Table.Cell>{group.name}</Table.Cell>
                                <Table.Cell class="text-center">
                                    <InputCheckbox
                                        ariaLabel={ta.pam.member}
                                        bind:checked={group.isMember}
                                        disabled={group.name === user.name}
                                    />
                                </Table.Cell>
                                {#if withWheel}
                                    <Table.Cell class="text-center">
                                        <InputCheckbox
                                            ariaLabel="Wheel"
                                            bind:checked={group.isWheel}
                                            disabled={!group.isMember}
                                        />
                                    </Table.Cell>
                                {/if}
                            </Table.Row>
                        {/each}
                    </Table.Body>
                </Table.Root>
            </div>
        </div>
    {/if}
{/snippet}

<div class="max-w-2xl">
    <Form action={url} {onSubmit}>
        <div class="grid gap-4 sm:grid-cols-2">
            <LabeledValue label="UID">
                {user.id}
            </LabeledValue>
            <LabeledValue label="GID">
                {user.gid}
            </LabeledValue>
            <LabeledValue label={ta.pam.username}>
                {user.name}
            </LabeledValue>
            <LabeledValue label="E-Mail">
                {user.email}
            </LabeledValue>
        </div>

        <div class="grid gap-4 sm:grid-cols-2">
            <Input label="Shell" placeholder="Shell" required bind:value={user.shell} />
            <Input label="Home" placeholder="Home" required bind:value={user.home_dir} />
        </div>

        <h2 class="text-lg font-semibold">{ta.pam.groups}</h2>

        <div class="grid gap-4">
            {@render snipGroups('Host', groupsHost, true)}
            {@render snipGroups('Generic', groupsGeneric)}
            {@render snipGroups('Local', groupsLocal)}
            {@render snipGroups('User', groupsUser)}
        </div>

        {#if details?.authorized_keys}
            <PamAuthorizedKeys
                authorizedKeys={details.authorized_keys}
                onSave={fetchDetails}
                isAdmin
                pamUid={user.id}
            />
        {/if}

        {#if deleteConfirm}
            <div class="grid gap-2 rounded-xl border p-3">
                <p class="text-sm">{ta.pam.deleteUser}</p>
                <div class="flex gap-2">
                    <Button level={-1} onclick={deleteUser}>
                        {t.common.delete}
                    </Button>
                    <Button level={3} onclick={() => (deleteConfirm = false)}>
                        {t.common.cancel}
                    </Button>
                </div>
            </div>
        {:else}
            <div class="flex flex-wrap items-center gap-2">
                <Button type="submit">
                    {t.common.save}
                </Button>
                <Button level={-1} onclick={() => (deleteConfirm = true)}>
                    {t.common.delete}
                </Button>
                <Msg {success} {err} />
            </div>
        {/if}
    </Form>
</div>
