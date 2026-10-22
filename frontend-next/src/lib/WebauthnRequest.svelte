<script lang="ts">
    import Loading from '$lib5/Loading.svelte';
    import { webauthnAuth, type WebauthnAuthResult } from '$mfa/webauthn/authentication';
    import { useI18n } from '$state/i18n.svelte.js';
    import type { WebauthnAdditionalData } from '$mfa/webauthn/types.ts';
    import { onMount } from 'svelte';
    import type { MfaPurpose } from '$api/types/mfa';

    let {
        purpose,
        resetUserId,
        onError,
        onSuccess,
    }: {
        purpose: MfaPurpose;
        resetUserId?: string;
        onError: (error: string) => void;
        onSuccess: (res?: WebauthnAdditionalData) => void;
    } = $props();

    let t = useI18n();

    let webauthnRes: undefined | WebauthnAuthResult = $state();

    onMount(async () => {
        webauthnRes = await webauthnAuth(
            purpose,
            t.authorize.invalidKeyUsed,
            t.authorize.requestExpired,
            resetUserId,
        );
    });

    $effect(() => {
        if (webauthnRes) {
            if (webauthnRes.error) {
                setTimeout(() => {
                    onError(webauthnRes?.error || 'Webauthn Error');
                }, 3000);
            } else {
                onSuccess(webauthnRes.data);
            }
        }
    });
</script>

{#if purpose}
    <div class="bg-card grid justify-items-center gap-3 rounded-xl border p-4 text-center">
        <div class="font-semibold">
            {t.authorize.expectingPasskey}
        </div>

        <!--
        We can't show an expiry visualization based on JS because an open PIN / key request window
        will block the JS event loop -> stick with just loading.
        -->
        {#if !webauthnRes}
            <Loading />
        {/if}

        {#if webauthnRes}
            {#if webauthnRes.error}
                <p class="text-destructive text-sm">
                    {webauthnRes.error}
                </p>
            {:else}
                <p class="text-primary text-sm">
                    {t.authorize.mfaAck}
                </p>
            {/if}
        {/if}
    </div>
{/if}
