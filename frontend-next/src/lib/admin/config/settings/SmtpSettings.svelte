<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type {
        SettingsResponse,
        SettingsTestResponse,
        SmtpSettingsRequest,
    } from '$api/types/settings';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputPassword from '$lib5/form/InputPassword.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import Msg from '$lib5/Msg.svelte';
    import { Badge } from '$lib/components/ui/badge';

    let settings: SettingsResponse | undefined = $state();

    let url = $state('');
    let port = $state('');
    let username = $state('');
    // empty = keep the stored password
    let password = $state('');
    let clearPassword = $state(false);
    let from = $state('');
    let tlsMode = $state('tls');

    let busy = $state(false);
    let err = $state('');
    let ok = $state('');

    const tlsItems = [
        { value: 'tls', label: 'TLS (port 465)' },
        { value: 'starttls', label: 'STARTTLS (port 587)' },
        { value: 'danger-insecure', label: 'None / insecure (local test servers only)' },
    ];

    onMount(load);

    async function load() {
        let res = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (res.body) {
            settings = res.body;
            const s = res.body.smtp;
            url = s.url ?? '';
            port = s.port ? String(s.port) : '';
            username = s.username ?? '';
            password = '';
            clearPassword = false;
            from = s.from;
            tlsMode = s.tls_mode;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function save() {
        err = '';
        ok = '';
        busy = true;
        let payload: SmtpSettingsRequest = {
            url: url.trim() || undefined,
            port: port.trim() ? Number(port) : undefined,
            username: username.trim() || undefined,
            from: from.trim(),
            tls_mode: tlsMode,
        };
        if (clearPassword) payload.password = '';
        else if (password) payload.password = password;

        let res = await fetchPut('/auth/v1/settings/smtp', payload);
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Email settings saved');
        await load();
    }

    async function test() {
        err = '';
        ok = '';
        busy = true;
        let res = await fetchPost<SettingsTestResponse>('/auth/v1/settings/smtp/test');
        busy = false;
        if (res.error) err = res.error.message;
        else ok = res.body?.message ?? 'Sent';
    }

    async function importFromFile() {
        err = '';
        busy = true;
        let res = await fetchPost('/auth/v1/settings/import/smtp');
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Imported from the config file');
        await load();
    }
</script>

<div class="grid max-w-2xl gap-4">
    <div class="flex flex-wrap items-center gap-3">
        <h3 class="text-xl font-semibold">Email (SMTP)</h3>
        {#if settings}
            {#if !settings.smtp_configured}
                <Badge variant="outline" class="text-amber-600 dark:text-amber-400">Not set up</Badge>
            {:else if settings.smtp.from_db}
                <Badge variant="secondary">Stored in the database</Badge>
            {:else}
                <Badge variant="outline">From the config file</Badge>
            {/if}
        {/if}
    </div>

    <p class="text-muted-foreground text-sm">
        The mail server Rauthy sends password resets, verification codes and admin messages through.
        Rauthy starts and works without it; emails are only skipped until it is set up. Changes apply
        immediately, no restart needed.
    </p>

    <div class="grid gap-4">
        <div class="grid gap-4 sm:grid-cols-[1fr_8rem]">
            <Input bind:value={url} label="Server" placeholder="smtp.example.com" autocomplete="off" />
            <Input bind:value={port} label="Port" placeholder="465" typ="text" autocomplete="off" />
        </div>
        <InputSelect bind:value={tlsMode} label="Encryption" items={tlsItems} required />
        <div class="grid gap-4 sm:grid-cols-2">
            <Input bind:value={username} label="Username" autocomplete="off" />
            <div class="grid gap-1">
                <InputPassword
                    bind:value={password}
                    label="Password"
                    ariaLabel="Password"
                    placeholder={settings?.smtp.has_password ? 'unchanged' : 'Password'}
                    autocomplete="new-password"
                    name="smtp-password"
                />
                {#if settings?.smtp.has_password}
                    <label class="text-muted-foreground flex items-center gap-2 text-xs">
                        <input type="checkbox" bind:checked={clearPassword} /> Remove the stored password
                    </label>
                {/if}
            </div>
        </div>
        <Input
            bind:value={from}
            label="Sender"
            placeholder="Rauthy <rauthy@example.com>"
            autocomplete="off"
            required
        />
    </div>

    <div class="flex flex-wrap items-center gap-2">
        <Button onclick={save} isLoading={busy}>Save</Button>
        <Button level={2} onclick={test} isDisabled={busy || !settings?.smtp_configured}>
            Send test email to me
        </Button>
        <Button level={3} onclick={importFromFile} isDisabled={busy}>Import from config file</Button>
    </div>
    <Msg {err} success={!!ok} />
    {#if ok}<p class="text-sm">{ok}</p>{/if}
</div>
