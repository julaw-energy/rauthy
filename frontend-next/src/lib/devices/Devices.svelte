<script lang="ts">
    import { useI18n } from '$state/i18n.svelte.js';
    import { fetchGet } from '$api/fetch';
    import type { DeviceResponse } from '$api/types/device.ts';
    import Device from './Device.svelte';

    let {
        userId,
    }: {
        userId: string;
    } = $props();

    let t = useI18n();

    let devices: DeviceResponse[] = $state([]);

    $effect(() => {
        fetchDevices();
    });

    async function fetchDevices() {
        let res = await fetchGet<DeviceResponse[]>(`/auth/v1/users/${userId}/devices`);
        if (res.body) {
            devices = res.body;
        } else {
            console.error('error fetching devices: ' + res.error);
        }
    }
</script>

<div class="grid max-w-2xl gap-3">
    <p class="text-muted-foreground text-sm">
        {t?.account.devicesDesc || 'Devices linked to this account'}
    </p>

    {#if userId}
        {#each devices as device, i (device.id)}
            <Device bind:device={devices[i]} {userId} />
        {/each}
    {/if}
</div>
