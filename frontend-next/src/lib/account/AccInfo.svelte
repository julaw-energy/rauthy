<script lang="ts">
    import CheckIcon from '$lib5/CheckIcon.svelte';
    import { buildWebIdUri, formatDateFromTs, saveProviderToken } from '$utils/helpers';
    import Button from '$lib5/button/Button.svelte';
    import Modal from '$lib5/Modal.svelte';
    import { PKCE_VERIFIER_UPSTREAM, TPL_IS_OTP_ENABLED } from '$utils/constants';
    import { useI18n } from '$state/i18n.svelte.js';
    import type { UserResponse } from '$api/types/user.ts';
    import type {
        AuthProvidersTemplate,
        AuthProviderTemplate,
    } from '$api/templates/AuthProvider.ts';
    import type { WebIdResponse } from '$api/types/web_id.ts';
    import type { ProviderLoginRequest } from '$api/types/auth_provider.ts';
    import { fetchDelete, fetchGet, fetchPost } from '$api/fetch';
    import ButtonAuthProvider from '../ButtonAuthProvider.svelte';
    import UserPicture from '$lib/UserPicture.svelte';
    import { fetchSolvePow } from '$utils/pow';
    import { generatePKCE } from '$utils/pkce';
    import type { PamUserResponse } from '$api/types/pam';
    import Template from '$lib5/Template.svelte';
    import type { OtpResponse } from '$api/types/otp';

    let {
        user = $bindable(),
        pamUser,
        providers,
        authProvider,
        webIdData,
        viewModePhone,
    }: {
        user: UserResponse;
        pamUser: undefined | PamUserResponse;
        providers: AuthProvidersTemplate;
        authProvider: undefined | AuthProviderTemplate;
        viewModePhone?: boolean;
        webIdData: undefined | WebIdResponse;
    } = $props();

    let t = useI18n();

    let unlinkErr = $state(false);
    let showModal = $state(false);

    let isFederated = $derived(user.account_type?.startsWith('federated'));
    let accType = $derived(
        isFederated ? `${user.account_type}: ${authProvider?.name || ''}` : user.account_type,
    );

    let classRow: 'rowPhone' | 'row' = $derived(viewModePhone ? 'rowPhone' : 'row');
    let classLabel: 'labelPhone' | 'label' = $derived(viewModePhone ? 'labelPhone' : 'label');

    let isLoading = $state(false);

    let fallbackCharacters = $derived.by(() => {
        let chars = '';
        if (user.given_name) {
            chars = user.given_name[0];
        } else {
            chars = user.email[0];
        }
        if (user.family_name && user.family_name.length > 0) {
            chars += user.family_name[0];
        }
        return chars;
    });

    let isOtpEnabled = $state(false);
    let hasOtp = $state(false);

    $effect(() => {
        if (isOtpEnabled) {
            fetchOtps();
        }
    });

    async function fetchOtps() {
        let res = await fetchGet<OtpResponse[]>(`/auth/v1/users/${user.id}/otp`);
        if (res.body) {
            for (let otp of res.body) {
                if (otp.is_active) {
                    hasOtp = true;
                    return;
                }
            }
        }
    }

    function linkProvider(id: string) {
        generatePKCE().then(pkce => {
            if (pkce) {
                localStorage.setItem(PKCE_VERIFIER_UPSTREAM, pkce.verifier);
                providerLoginPkce(id, pkce.challenge);
            }
        });
    }

    async function providerLoginPkce(id: string, pkce_challenge: string) {
        isLoading = true;

        let pow = (await fetchSolvePow()) || '';
        let payload: ProviderLoginRequest = {
            email: user.email,
            pow,
            client_id: 'rauthy',
            redirect_uri: window.location.href,
            provider_id: id,
            pkce_challenge,
        };

        let res = await fetchPost<string>(`/auth/v1/providers/${id}/link`, payload);
        isLoading = false;

        if (res.text) {
            saveProviderToken(res.text);
            let loc = res.headers.get('location');
            if (loc) {
                window.location.href = loc;
            }
        } else {
            console.error(res.error);
        }
    }

    async function unlinkProvider() {
        let res = await fetchDelete<UserResponse>('/auth/v1/providers/link');
        if (res.body) {
            user = res.body;
        } else {
            console.error(res.error);
            unlinkErr = true;
        }
    }
</script>

<Template id={TPL_IS_OTP_ENABLED} bind:value={isOtpEnabled} />

{#snippet row(label: string, value?: string)}
    <div class="grid grid-cols-[minmax(7rem,35%)_1fr] items-start gap-x-3 gap-y-1 border-b py-2 last:border-b-0">
        <dt class="text-muted-foreground text-sm">{label}</dt>
        <dd class="min-w-0 text-sm break-words">{value ?? ''}</dd>
    </div>
{/snippet}

<div class="grid gap-2">
    <div class="flex justify-center py-1">
        <UserPicture
            {fallbackCharacters}
            userId={user.id}
            bind:pictureId={user.picture_id}
            size="large"
        />
    </div>

    <dl class="grid">
        {@render row(t.common.email, user.email)}

        {#if pamUser}
            {@render row(`PAM ${t.account.user}`, pamUser.name)}
        {/if}
        {#if user.given_name}
            {@render row(t.account.givenName, user.given_name)}
        {/if}
        {#if user.family_name}
            {@render row(t.account.familyName, user.family_name)}
        {/if}

        {@render row(`${t.account.user} ID`, user.id)}

        <div class="grid grid-cols-[minmax(7rem,35%)_1fr] items-start gap-x-3 gap-y-2 border-b py-1">
            <dt class="text-muted-foreground text-sm">{t.account.accType}</dt>
            <dd class="grid min-w-0 gap-2 text-sm">
                <span>{accType || ''}</span>
                {#if isFederated}
                    <div class="grid gap-1">
                        <div>
                            <Button ariaLabel={t.account.providerUnlink} level={2} onclick={unlinkProvider}>
                                {t.account.providerUnlink}
                            </Button>
                        </div>
                        {#if unlinkErr}
                            <p class="text-destructive text-xs">{t.account.providerUnlinkDesc}</p>
                        {/if}
                    </div>
                {:else if providers.length > 0}
                    <div>
                        <Button level={2} onclick={() => (showModal = true)}>
                            {t.account.providerLink}
                        </Button>
                    </div>
                    <Modal bind:showModal>
                        <h3 class="text-lg font-semibold">{t.account.providerLink}</h3>
                        <p class="text-muted-foreground text-sm">{t.account.providerLinkDesc}</p>

                        <div class="grid gap-2">
                            {#each providers as provider (provider.id)}
                                <ButtonAuthProvider
                                    ariaLabel={`${t.account.providerLink}: ${provider.name}`}
                                    {provider}
                                    onclick={linkProvider}
                                    {isLoading}
                                />
                            {/each}
                        </div>
                    </Modal>
                {/if}
            </dd>
        </div>

        {@render row(t.account.roles, user.roles.join(', ') || 'None')}
        {@render row(t.account.groups, user.groups?.join(', ') || 'None')}

        <div class="grid grid-cols-[minmax(7rem,35%)_1fr] items-center gap-x-3 border-b py-1">
            <dt class="text-muted-foreground text-sm">{t.account.mfaActivated}</dt>
            <dd><CheckIcon checked={!!user.webauthn_user_id || hasOtp} /></dd>
        </div>
        <div class="grid grid-cols-[minmax(7rem,35%)_1fr] items-center gap-x-3 border-b py-1">
            <dt class="text-muted-foreground text-sm">{t.account.userEnabled}</dt>
            <dd><CheckIcon checked={user.enabled} /></dd>
        </div>
        <div class="grid grid-cols-[minmax(7rem,35%)_1fr] items-center gap-x-3 border-b py-1">
            <dt class="text-muted-foreground text-sm">{t.account.emailVerified}</dt>
            <dd><CheckIcon checked={user.email_verified} /></dd>
        </div>

        {#if user.last_login}
            {@render row(t.account.lastLogin, formatDateFromTs(user.last_login))}
        {/if}
        {@render row(
            t.account.passwordExpiry,
            (user.password_expires && formatDateFromTs(user.password_expires)) || t.common.never,
        )}
        {@render row(t.account.userCreated, formatDateFromTs(user.created_at))}
        {#if user.user_expires}
            {@render row(t.account.userExpiry, formatDateFromTs(user.user_expires))}
        {/if}

        {#if webIdData}
            <div class="grid grid-cols-[minmax(7rem,35%)_1fr] items-start gap-x-3 py-1">
                <dt class="text-muted-foreground text-sm">WebID:</dt>
                <dd class="min-w-0 text-sm break-all">
                    <a class="text-primary hover:underline" href={buildWebIdUri(user.id)} target="_blank">
                        {buildWebIdUri(user.id)}
                    </a>
                </dd>
            </div>
        {/if}
    </dl>
</div>
