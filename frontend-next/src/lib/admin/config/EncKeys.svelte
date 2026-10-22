<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { onMount } from 'svelte';
    import Button from '$lib5/button/Button.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import { fetchGet, fetchPost } from '$api/fetch';
    import type { EncKeyMigrateRequest, EncKeysResponse } from '$api/types/enc_keys.ts';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Options from '$lib5/Options.svelte';

    let ta = useI18nAdmin();

    let err = $state('');
    let isLoading = $state(true);
    let activeKey = $state('');
    let migrateKey = $state('');
    let keys: string[] = $state([]);
    let success = $state(false);

    onMount(async () => {
        let res = await fetchGet<EncKeysResponse>('/auth/v1/encryption/keys');
        if (res.body) {
            keys = res.body.keys;
            activeKey = res.body.active;
            migrateKey = res.body.active;
        } else {
            err = res.error?.message || 'Error';
        }
        isLoading = false;
    });

    async function migrate() {
        isLoading = true;

        let payload: EncKeyMigrateRequest = {
            key_id: migrateKey,
        };
        let res = await fetchPost('/auth/v1/encryption/migrate', payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            setTimeout(() => {
                success = false;
            }, 4000);
        }

        isLoading = false;
    }
</script>

<div class="grid max-w-3xl gap-4">
    <h3 class="text-xl font-semibold">{ta.docs.encKeys.header}</h3>

    <div class="text-muted-foreground grid gap-2 text-sm">
        <p>{ta.docs.encKeys.p1}</p>
        <p>{ta.docs.encKeys.p2}</p>
    </div>

    <p>
        <span class="font-medium">{ta.docs.encKeys.keyActive}:</span>
        <span class="text-primary font-mono">{activeKey}</span>
    </p>

    <div class="grid gap-1">
        <p class="font-medium">{ta.docs.encKeys.keysAvailable}:</p>
        <ul class="list-disc pl-5">
            {#each keys as key (key)}
                <li class="font-mono">{key}</li>
            {/each}
        </ul>
    </div>

    <p class="text-muted-foreground text-sm">{ta.docs.encKeys.p3}</p>

    {#if keys.length > 1}
        <p class="font-medium">{ta.docs.encKeys.migrateToKey}:</p>

        <div class="max-w-sm">
            <Options ariaLabel={ta.docs.encKeys.migrateToKey} options={keys} bind:value={migrateKey} />
        </div>

        <div class="flex items-center gap-3">
            <Button onclick={migrate} {isLoading}>
                {ta.docs.encKeys.migrate}
            </Button>
            <Msg {success} />
        </div>
    {:else}
        <p class="text-destructive text-sm">{ta.docs.encKeys.pNotPossible}</p>
    {/if}

    <Msg {err} />
</div>
