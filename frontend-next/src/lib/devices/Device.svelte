<script lang="ts">
    import type { DeviceRequest, DeviceResponse } from '$api/types/device.ts';
    import { formatDateFromTs } from '$utils/helpers';
    import IconStop from '$icons/IconStop.svelte';
    import Input from '$lib5/form/Input.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { untrack } from 'svelte';
    import Expandable from '$lib5/Expandable.svelte';
    import { PATTERN_CLIENT_NAME } from '$utils/patterns';
    import LabeledValue from '$lib5/LabeledValue.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { fetchDelete, fetchPut } from '$api/fetch';

    let {
        device = $bindable(),
        userId,
    }: {
        device: DeviceResponse;
        userId: string;
    } = $props();

    let t = useI18n();

    let name = $state(untrack(() => device.name));

    async function onSaveName() {
        let payload: DeviceRequest = {
            device_id: device.id,
            name,
        };
        let res = await fetchPut(`/auth/v1/users/${userId}/devices`, payload);
        if (!res.error) {
            // update the name in memory to blend out the save button
            device.name = name;
        } else {
            console.error(res.error);
        }
    }

    async function onRevokeRefresh() {
        let payload: DeviceRequest = {
            device_id: device.id,
        };
        let res = await fetchDelete(`/auth/v1/users/${userId}/devices`, payload);
        if (!res.error) {
            device.refresh_exp = undefined;
        } else {
            console.error(res.error);
        }
    }
</script>

<div class="rounded-xl border">
    <Expandable>
        {#snippet summary()}
            <span class="font-mono">{device.name}</span>
        {/snippet}

        {#snippet details()}
            <div class="grid gap-4 p-1">
                <LabeledValue label={t.account.deviceId} mono>
                    {device.id}
                </LabeledValue>

                <div class="flex flex-wrap items-end gap-2">
                    <div class="min-w-56 flex-1">
                        <Input
                            bind:value={name}
                            autocomplete="off"
                            label={t.account.deviceName}
                            placeholder={t.account.deviceName}
                            onEnter={onSaveName}
                            pattern={PATTERN_CLIENT_NAME}
                            required
                        />
                    </div>
                    {#if name !== device.name}
                        <Button onclick={onSaveName}>
                            {t.common.save}
                        </Button>
                    {/if}
                </div>

                <div class="grid gap-4 sm:grid-cols-2">
                    <LabeledValue label={t.account.regDate}>
                        {formatDateFromTs(device.created)}
                    </LabeledValue>

                    <LabeledValue label={t.account.accessExp}>
                        {formatDateFromTs(device.access_exp)}
                    </LabeledValue>

                    {#if device.refresh_exp}
                        <LabeledValue label={t.account.accessRenew}>
                            {formatDateFromTs(device.refresh_exp)}
                            {#snippet button()}
                                <Button ariaLabel={t.common.delete} invisible level={-3} onclick={onRevokeRefresh}>
                                    <span title={t.common.delete}>
                                        <IconStop width="1.2rem" />
                                    </span>
                                </Button>
                            {/snippet}
                        </LabeledValue>
                    {/if}

                    <LabeledValue label={t.account.regIp}>
                        {device.peer_ip}
                    </LabeledValue>
                </div>
            </div>
        {/snippet}
    </Expandable>
</div>
