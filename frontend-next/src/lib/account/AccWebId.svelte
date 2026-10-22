<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { fade, slide } from 'svelte/transition';
    import Switch from '$lib5/Switch.svelte';
    import { buildWebIdUri } from '$utils/helpers';
    import { useI18n } from '$state/i18n.svelte.js';
    import type { WebIdResponse } from '$api/types/web_id.ts';
    import A from '$lib5/A.svelte';
    import InputArea from '$lib5/form/InputArea.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import { fetchPut } from '$api/fetch';

    let {
        webIdData = $bindable(),
    }: {
        webIdData: WebIdResponse;
    } = $props();

    let t = useI18n();

    const labelWidth = '14rem';

    let err = $state('');
    let success = $state(false);
    let expertMode = $state(!!webIdData.custom_triples);
    let webIdLink = buildWebIdUri(webIdData.user_id);

    async function onSubmit() {
        err = '';

        let res = await fetchPut(`/auth/v1/users/${webIdData.user_id}/webid/data`, webIdData);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            setTimeout(() => {
                success = false;
            }, 3000);
        }
    }
</script>

<div class="grid max-w-2xl gap-4">
    <p class="text-muted-foreground text-sm">{t.account.webIdDesc}</p>

    <p class="text-sm break-all">
        <A href={webIdLink} target="_blank">
            {webIdLink}
        </A>
    </p>

    <Switch ariaLabel="E-Mail" bind:checked={webIdData.expose_email} {labelWidth}>
        E-Mail
    </Switch>

    <Switch ariaLabel={t.account.webIdExpertMode} bind:checked={expertMode} {labelWidth}>
        {t.account.webIdExpertMode}
    </Switch>

    {#if expertMode}
        <div class="grid gap-2" transition:slide={{ duration: 150 }}>
            <p class="text-muted-foreground text-sm">{t.account.webIdDescData}</p>
            <InputArea placeholder="FOAF" rows={12} fontMono bind:value={webIdData.custom_triples} />
        </div>
    {/if}

    <div class="flex items-center gap-3">
        <Button onclick={onSubmit}>
            {t.common.save}
        </Button>
        <Msg {success} {err} />
    </div>
</div>
