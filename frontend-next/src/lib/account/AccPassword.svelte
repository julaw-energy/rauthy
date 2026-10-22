<script lang="ts">
    import Button from '$lib5/button/Button.svelte';
    import { fade } from 'svelte/transition';
    import AccModPwd from '$lib5/account/AccModPwd.svelte';
    import WebauthnRequest from '$lib5/WebauthnRequest.svelte';
    import CheckIcon from '$lib5/CheckIcon.svelte';
    import { useI18n } from '$state/i18n.svelte.js';
    import type { UpdateUserSelfRequest, UserResponse } from '$api/types/user.ts';
    import type { AuthProviderTemplate } from '$api/templates/AuthProvider.ts';
    import IconCheck from '$icons/IconCheck.svelte';
    import type { PropsPassword } from './props.ts';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type { WebauthnAdditionalData } from '$mfa/webauthn/types.js';
    import type { PasskeyResponse } from '$api/types/webauthn.ts';
    import { onMount } from 'svelte';
    import type { RequestResetRequest } from '$api/types/authorize.ts';
    import { fetchSolvePow } from '$utils/pow';
    import type { MfaPurpose } from '$api/types/mfa.js';

    let {
        user = $bindable(),
        authProvider,
        viewModePhone,
    }: {
        user: UserResponse;
        authProvider: undefined | AuthProviderTemplate;
        viewModePhone?: boolean;
    } = $props();

    let t = useI18n();

    let inputWidth = $derived(viewModePhone ? 'calc(100vw - 1.5rem)' : '300px');

    let accType = $state(user.account_type);
    let passkeys: PasskeyResponse[] = $state([]);
    let isPwdValid: undefined | (() => boolean) = $state();
    let convertAccount = $state(false);
    let isLoading = $state(false);
    let err = $state('');
    let success = $state(false);
    let mfaPurpose: undefined | MfaPurpose = $state();

    let passwords: PropsPassword = $state({
        current: '',
        new: '',
        newConfirm: '',
    });

    let canConvertToPasskey = $derived(passkeys.filter(pk => pk.user_verified).length > 0);

    onMount(() => {
        fetchPasskeys();
    });

    async function fetchPasskeys() {
        let res = await fetchGet<PasskeyResponse[]>(`/auth/v1/users/${user.id}/webauthn`);
        if (res.body) {
            passkeys = res.body;
        } else {
            console.error('error fetching passkeys: ' + res.error);
        }
    }

    async function convertToPasskeyOnly() {
        let res = await fetchPost<UserResponse>(`/auth/v1/users/${user.id}/self/convert_passkey`);
        if (!res.error) {
            window.location.reload();
        } else {
            console.error('error fetching passkeys: ' + res.error);
        }
    }

    async function onSubmit() {
        err = '';
        if (passkeys.length > 0) {
            await onSubmitMfa();
        } else {
            await onSubmitFinish();
        }
    }

    async function onSubmitMfa() {
        if (!isPwdValid?.()) {
            err = t.common.invalidInput;
            return;
        }
        mfaPurpose = 'PasswordNew';
    }

    async function onSubmitFinish(mfaCode?: string) {
        if (!isPwdValid?.()) {
            err = t.common.invalidInput;
            return;
        }

        isLoading = true;

        let payload: UpdateUserSelfRequest = {
            password_new: passwords.new,
        };
        if (mfaCode) {
            payload.mfa_code = mfaCode;
        } else {
            payload.password_current = passwords.current;
        }

        let res = await fetchPut<UserResponse>(`/auth/v1/users/${user.id}/self`, payload);
        if (res.body) {
            success = true;
            passwords = {
                current: '',
                new: '',
                newConfirm: '',
            };
            user = res.body;
            accType = res.body.account_type;

            setTimeout(() => {
                success = false;
                convertAccount = false;
            }, 3000);
        } else {
            err = res.error?.message || 'Error';
        }

        isLoading = false;
    }

    function onWebauthnError(error: string) {
        mfaPurpose = undefined;
        err = error;
        setTimeout(() => {
            err = '';
        }, 5000);
    }

    function onWebauthnSuccess(data?: WebauthnAdditionalData) {
        mfaPurpose = undefined;
        if (data && 'code' in data) {
            onSubmitFinish(data.code);
        }
    }

    async function requestPasswordReset() {
        let pow = (await fetchSolvePow()) || '';
        let payload: RequestResetRequest = {
            pow,
            email: user.email,
        };
        let res = await fetchPost('/auth/v1/users/request_reset', payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
        }
    }
</script>

<div class="grid max-w-xl gap-6">
    {#if mfaPurpose}
        <WebauthnRequest
            purpose={mfaPurpose}
            onSuccess={onWebauthnSuccess}
            onError={onWebauthnError}
        />
    {/if}

    {#if accType === 'federated'}
        <div class="grid gap-2">
            <p class="text-sm">{t.account.federatedConvertPassword1}</p>
            <p class="font-semibold">{authProvider?.name || 'UNKNOWN'}</p>
            <p class="text-muted-foreground text-sm">{t.account.federatedConvertPassword2}</p>
            {#if success}
                <CheckIcon checked />
            {:else}
                <div>
                    <Button level={2} onclick={requestPasswordReset}>
                        {t.account.passwordReset}
                    </Button>
                </div>
            {/if}
        </div>
    {/if}

    {#if (accType === 'passkey' || accType === 'federated_passkey') && !convertAccount}
        <div class="text-muted-foreground grid gap-2 text-sm">
            <p>{t.account.accTypePasskeyText1}</p>
            <p>{t.account.accTypePasskeyText2}</p>
            <p>{t.account.accTypePasskeyText3}</p>
        </div>
        <div>
            <Button level={2} onclick={() => (convertAccount = true)}>
                {t.account.convertAccount}
            </Button>
        </div>
    {/if}

    {#if accType === 'password' || accType === 'federated_password' || convertAccount}
        <div class="grid gap-4">
            <AccModPwd
                bind:passwords
                bind:isValid={isPwdValid}
                {inputWidth}
                hideCurrentPassword={!(accType === 'password' && passkeys.length < 1)}
            />

            <div class="flex flex-wrap items-center gap-3">
                <Button onclick={onSubmit} level={1} {isLoading}>
                    {t.common.save}
                </Button>
                {#if success}
                    <span transition:fade><IconCheck width="1.4rem" /></span>
                {:else if err}
                    <p class="text-destructive text-sm" transition:fade>{err}</p>
                {:else if convertAccount && !isLoading}
                    <Button level={3} onclick={() => (convertAccount = false)}>
                        {t.common.cancel}
                    </Button>
                {/if}
            </div>
        </div>

        {#if !convertAccount && canConvertToPasskey}
            <div class="grid gap-2 rounded-xl border p-4">
                <h3 class="font-semibold">{t.account.convertAccount}</h3>
                <p class="text-muted-foreground text-sm">{t.account.convertAccountP1}</p>
                <div>
                    <Button level={2} onclick={convertToPasskeyOnly}>
                        {t.account.convertAccount}
                    </Button>
                </div>
            </div>
        {/if}
    {/if}
</div>
