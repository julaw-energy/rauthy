<script lang="ts">
    import { onMount } from 'svelte';
    import { fetchGet } from '$api/fetch';
    import type { ChannelSettingsResponse, SettingsResponse } from '$api/types/settings';
    import type { UserAttrConfigResponse } from '$api/types/user_attrs';
    import Button from '$lib5/button/Button.svelte';
    import ChannelEditor from './ChannelEditor.svelte';

    let channels: ChannelSettingsResponse[] = $state([]);
    let attrs: string[] = $state([]);
    let adding = $state(false);
    // forces the editors to start over after a reload, so they show what is stored
    let round = $state(0);

    onMount(load);

    async function load() {
        let [res, a] = await Promise.all([
            fetchGet<SettingsResponse>('/auth/v1/settings'),
            fetchGet<UserAttrConfigResponse>('/auth/v1/users/attr'),
        ]);
        if (a.body) attrs = a.body.values.map(v => v.name);
        if (res.body) channels = res.body.channels;
        round += 1;
    }
</script>

<div class="grid max-w-3xl gap-6">
    <div class="grid gap-2">
        <h3 class="text-xl font-semibold">Message channels</h3>
        <p class="text-muted-foreground text-sm">
            <b>Email</b> and <b>SMS</b> use the email address and the phone number every user has. Add a
            channel for anything else, like a WhatsApp username: it is tied to a user attribute, and a
            user without that value gets nothing through it.
        </p>
        <p class="text-muted-foreground text-sm">
            A channel delivers through <b>SMTP</b> (the built-in mailer, or its own server with its own
            credentials), a <b>webhook</b> (Rauthy's signed format, for a communication agent you run) or
            <b>HTTP</b> (you describe the call to any provider). Everything, including the system emails,
            follows the channel, so the SMTP server can be swapped for an agent without touching anything
            else. Changes apply immediately.
        </p>
        <p class="text-muted-foreground text-sm">
            A channel can verify user values: link it to a verifiable attribute under Attributes.
        </p>
    </div>

    {#key round}
        {#each channels as c (c.channel)}
            <ChannelEditor channel={c} {attrs} onChange={load} />
        {/each}

        {#if adding}
            <ChannelEditor {attrs} onChange={load} onCancel={() => (adding = false)} />
        {:else}
            <div>
                <Button level={2} onclick={() => (adding = true)}>Add a channel</Button>
            </div>
        {/if}
    {/key}
</div>
