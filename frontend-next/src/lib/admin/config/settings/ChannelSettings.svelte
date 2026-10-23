<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type {
        ChannelSettingsRequest,
        ChannelSettingsResponse,
        MessageChannel,
        SettingsResponse,
        SettingsTestResponse,
    } from '$api/types/settings';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputPassword from '$lib5/form/InputPassword.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import { Badge } from '$lib/components/ui/badge';

    const labels: Record<MessageChannel, string> = {
        email: 'Email',
        sms: 'SMS',
        whatsapp: 'WhatsApp',
    };

    interface Draft {
        driver: string;
        url: string;
        apiKey: string;
        clearKey: boolean;
        channelName: string;
    }

    let channels: ChannelSettingsResponse[] = $state([]);
    let drafts: Record<string, Draft> = $state({});
    let busy = $state('');
    let msgs: Record<string, { err?: string; ok?: string }> = $state({});

    onMount(load);

    async function load() {
        let res = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (!res.body) return;
        channels = res.body.channels;
        for (const c of channels) {
            drafts[c.channel] = {
                driver: c.driver,
                url: c.url ?? '',
                apiKey: '',
                clearKey: false,
                channelName: c.channel_name,
            };
        }
    }

    const driverItems = (c: MessageChannel) => [
        { value: '', label: 'Disabled' },
        ...(c === 'email' ? [{ value: 'smtp', label: 'Built-in mailer (SMTP)' }] : []),
        { value: 'notifyd', label: 'notifyd' },
        { value: 'webhook', label: 'Generic webhook' },
    ];

    const usesAgent = (d: Draft) => d.driver === 'notifyd' || d.driver === 'webhook';

    async function save(c: MessageChannel) {
        const d = drafts[c];
        msgs[c] = {};
        busy = c;
        let payload: ChannelSettingsRequest = {
            driver: d.driver,
            url: d.url.trim() || undefined,
            channel_name: d.channelName.trim() || undefined,
        };
        if (d.clearKey) payload.api_key = '';
        else if (d.apiKey) payload.api_key = d.apiKey;

        let res = await fetchPut(`/auth/v1/settings/channel/${c}`, payload);
        busy = '';
        if (res.error) {
            msgs[c] = { err: res.error.message };
            return;
        }
        toast.success(`${labels[c]} channel saved`);
        await load();
    }

    async function test(c: MessageChannel) {
        msgs[c] = {};
        busy = c;
        let res = await fetchPost<SettingsTestResponse>(`/auth/v1/settings/channel/${c}/test`);
        busy = '';
        msgs[c] = res.error ? { err: res.error.message } : { ok: res.body?.message };
    }

    async function importFromFile(c: MessageChannel) {
        msgs[c] = {};
        busy = c;
        let res = await fetchPost(`/auth/v1/settings/import/channel.${c}`);
        busy = '';
        if (res.error) {
            msgs[c] = { err: res.error.message };
            return;
        }
        toast.success('Imported from the config file');
        await load();
    }
</script>

<div class="grid max-w-2xl gap-6">
    <div class="grid gap-2">
        <h3 class="text-xl font-semibold">Message channels</h3>
        <p class="text-muted-foreground text-sm">
            How Rauthy delivers messages besides the built-in mailer: SMS and WhatsApp for admin messages
            and verification codes. Rauthy sends every message in one fixed format, so the notification
            agent behind a channel can be swapped here without changing anything else. Changes apply
            immediately.
        </p>
    </div>

    {#each channels as c (c.channel)}
        {@const d = drafts[c.channel]}
        {#if d}
            <div class="bg-card grid gap-4 rounded-xl border p-4">
                <div class="flex flex-wrap items-center gap-3">
                    <h4 class="font-semibold">{labels[c.channel]}</h4>
                    {#if c.driver === ''}
                        <Badge variant="outline">Disabled</Badge>
                    {:else if c.from_db}
                        <Badge variant="secondary">Stored in the database</Badge>
                    {:else}
                        <Badge variant="outline">From the config file</Badge>
                    {/if}
                </div>

                <InputSelect bind:value={d.driver} label="Driver" items={driverItems(c.channel)} />

                {#if usesAgent(d)}
                    <Input
                        bind:value={d.url}
                        label={d.driver === 'notifyd' ? 'notifyd URL' : 'Webhook URL'}
                        placeholder="http://notifyd:3400"
                        autocomplete="off"
                    />
                    <div class="grid gap-4 sm:grid-cols-2">
                        <div class="grid gap-1">
                            <InputPassword
                                bind:value={d.apiKey}
                                label={d.driver === 'notifyd' ? 'API key' : 'Signing secret'}
                                ariaLabel="API key"
                                placeholder={c.has_api_key ? 'unchanged' : 'not set'}
                                autocomplete="new-password"
                                name={`key-${c.channel}`}
                            />
                            {#if c.has_api_key}
                                <label class="text-muted-foreground flex items-center gap-2 text-xs">
                                    <input type="checkbox" bind:checked={d.clearKey} /> Remove the stored key
                                </label>
                            {/if}
                        </div>
                        <Input
                            bind:value={d.channelName}
                            label="Channel name at the agent"
                            placeholder={c.channel}
                            autocomplete="off"
                        />
                    </div>
                {/if}

                <div class="flex flex-wrap items-center gap-2">
                    <Button onclick={() => save(c.channel)} isLoading={busy === c.channel}>Save</Button>
                    <Button
                        level={2}
                        onclick={() => test(c.channel)}
                        isDisabled={busy !== '' || !usesAgent({ ...d, driver: c.driver })}
                    >
                        Test the agent
                    </Button>
                    <Button level={3} onclick={() => importFromFile(c.channel)} isDisabled={busy !== ''}>
                        Import from config file
                    </Button>
                </div>
                {#if msgs[c.channel]?.err}
                    <p class="text-destructive text-sm">{msgs[c.channel].err}</p>
                {:else if msgs[c.channel]?.ok}
                    <p class="text-sm">{msgs[c.channel].ok}</p>
                {/if}
            </div>
        {/if}
    {/each}
</div>
