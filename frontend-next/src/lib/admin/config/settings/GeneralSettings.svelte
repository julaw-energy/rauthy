<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type { GeneralSettingsRequest, SettingsResponse } from '$api/types/settings';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import Msg from '$lib5/Msg.svelte';
    import { Badge } from '$lib/components/ui/badge';

    let settings: SettingsResponse | undefined = $state();
    let appName = $state('');
    let busy = $state(false);
    let err = $state('');

    onMount(load);

    async function load() {
        let res = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (res.body) {
            settings = res.body;
            appName = res.body.general.app_name;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function save() {
        err = '';
        busy = true;
        let payload: GeneralSettingsRequest = { app_name: appName.trim() };
        let res = await fetchPut('/auth/v1/settings/general', payload);
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Saved. Open pages pick the new name up on their next load.');
        // remember it for this browser so the next page load is branded at once
        try {
            localStorage.setItem('app_name', payload.app_name);
        } catch {
            // not important
        }
        await load();
    }

    async function importFromFile() {
        err = '';
        busy = true;
        let res = await fetchPost('/auth/v1/settings/import/general');
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
        <h3 class="text-xl font-semibold">General</h3>
        {#if settings}
            {#if settings.general.from_db}
                <Badge variant="secondary">Stored in the database</Badge>
            {:else}
                <Badge variant="outline">From the config file</Badge>
            {/if}
        {/if}
    </div>

    <p class="text-muted-foreground text-sm">
        The name of this service as your users see it: in page titles, the admin UI, the texts of the login and
        account pages, emails and the built-in documentation. It replaces the word "Rauthy" there. Passkey
        prompts use the <code>webauthn.rp_name</code> from the config file, which follows
        <code>server.app_name</code> unless set explicitly, and needs a restart.
    </p>

    <Input bind:value={appName} label="Name" placeholder="Rauthy" maxLength={48} autocomplete="off" required />

    <div class="flex flex-wrap items-center gap-2">
        <Button onclick={save} isLoading={busy} isDisabled={!appName.trim()}>Save</Button>
        <Button level={3} onclick={importFromFile} isDisabled={busy}>Import from config file</Button>
    </div>
    <Msg {err} />
</div>
