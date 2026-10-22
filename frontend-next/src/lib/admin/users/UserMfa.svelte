<script lang="ts">
    import type { UserResponse } from '$api/types/user.ts';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import type { PasskeyResponse } from '$api/types/webauthn.ts';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import UserPasskey from '$lib5/UserPasskey.svelte';
    import type { OtpResponse } from '$api/types/otp';
    import Template from '$lib5/Template.svelte';
    import { TPL_IS_OTP_ENABLED } from '$utils/constants';
    import { deleteOtp } from '$mfa/otp/mod';
    import UserOtp from '$lib5/UserOtp.svelte';

    let {
        user,
        onSave,
    }: {
        user: UserResponse;
        onSave: () => void;
    } = $props();

    let ta = useI18nAdmin();

    let err = $state('');
    let passkeys: PasskeyResponse[] = $state([]);

    let otps: OtpResponse[] = $state([]);
    let isOtpEnabled = $state(false);

    $effect(() => {
        fetchPasskeys();
    });

    $effect(() => {
        if (isOtpEnabled) {
            fetchOtps();
        }
    });

    async function fetchPasskeys() {
        let res = await fetchGet<PasskeyResponse[]>(`/auth/v1/users/${user.id}/webauthn`);
        if (res.body) {
            passkeys = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function onDelete(name: string) {
        let isLastKey = passkeys.length === 1;

        let res = await fetchDelete(
            `/auth/v1/users/${user.id}/webauthn/delete/${encodeURIComponent(name)}`,
            {},
        );
        if (res.status === 200) {
            await fetchPasskeys();

            // if this was the last key, we need to re-fetch the user to show the
            // correct "mfa enabled" status
            if (isLastKey) {
                onSave();
            }
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchOtps() {
        let res = await fetchGet<OtpResponse[]>(`/auth/v1/users/${user.id}/otp`);
        if (res.body) {
            otps = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function onDeleteOtp(id: string) {
        let res = await deleteOtp(user.id, id, undefined);
        if (res.error) {
            err = res.error || 'Error';
        } else {
            await fetchOtps();
        }
    }
</script>

<Template id={TPL_IS_OTP_ENABLED} bind:value={isOtpEnabled} />

<div class="grid max-w-2xl gap-4">
    {#if isOtpEnabled}
        <h3 class="font-semibold">{ta.users.mfa.webauthn.title}</h3>
    {/if}
    {#if passkeys.length === 0}
        <p class="text-muted-foreground text-sm">{ta.users.mfa.webauthn.noMfaKeys}</p>
    {:else}
        <div class="text-muted-foreground grid gap-1 text-sm">
            <p>{ta.users.mfa.webauthn.mfaDelete1}</p>
            <p>{@html ta.users.mfa.webauthn.mfaDelete2}</p>
        </div>

        <div class="grid gap-2">
            {#each passkeys as passkey (passkey.name)}
                <UserPasskey {passkey} showDelete {onDelete} />
            {/each}
        </div>
    {/if}

    <h3 class="font-semibold">{ta.users.mfa.otp.title}</h3>
    {#if otps.length === 0}
        <p class="text-muted-foreground text-sm">{ta.users.mfa.otp.noMfaOtps}</p>
    {:else}
        <div class="text-muted-foreground grid gap-1 text-sm">
            <p>{ta.users.mfa.otp.mfaDelete1}</p>
            <p>{@html ta.users.mfa.otp.mfaDelete2}</p>
        </div>

        <div class="grid gap-2">
            {#each otps as otp (otp.id)}
                <UserOtp {otp} showInactive={true} onDelete={onDeleteOtp} />
            {/each}
        </div>
    {/if}

    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}
</div>
