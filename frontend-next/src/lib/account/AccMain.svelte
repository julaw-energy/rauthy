<script lang="ts">
    import DateFormatSelect from '$lib5/DateFormatSelect.svelte';
    import * as Card from '$lib/components/ui/card';
    import { normalizeClientUri, redirectToLogout } from '$utils/helpers';
    import AccInfo from '$lib5/account/AccInfo.svelte';
    import AccEdit from '$lib5/account/AccEdit.svelte';
    import AccMFA from './AccMFA.svelte';
    import AccPassword from './AccPassword.svelte';
    import AccWebId from './AccWebId.svelte';
    import AccDevices from '$lib5/account/AccDevices.svelte';
    import { useI18n } from '$state/i18n.svelte.js';
    import type { UserResponse } from '$api/types/user.ts';
    import { TPL_AUTH_PROVIDERS, TPL_USER_VALUES_CONFIG } from '$utils/constants';
    import Template from '$lib5/Template.svelte';
    import type { AuthProvidersTemplate } from '$api/templates/AuthProvider.ts';
    import { onMount } from 'svelte';
    import { useParam } from '$state/param.svelte';
    import Tabs from '$lib5/tabs/Tabs.svelte';
    import Devices from '$lib5/devices/Devices.svelte';
    import type { WebIdResponse } from '$api/types/web_id.ts';
    import IconLogout from '$icons/IconLogout.svelte';
    import IconUserGroup from '$icons/IconUserGroup.svelte';
    import Button from '$lib/button/Button.svelte';
    import A from '$lib5/A.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import LangSelector from '$lib5/LangSelector.svelte';
    import { useSession } from '$state/session.svelte';
    import AccOther from '$lib/account/AccOther.svelte';
    import AccPAM from '$lib/account/AccPAM.svelte';
    import UserClients from '$lib/account/UserClients.svelte';
    import MissingFields from '$lib/account/MissingFields.svelte';
    import type { PamUserResponse } from '$api/types/pam';
    import { fetchGet, fetchPost } from '$api/fetch';
    import type { UserValuesConfig } from '$api/templates/UserValuesConfig';
    import IconHome from '$icons/IconHome.svelte';
    import IconArrowLeft from '$icons/IconArrowLeft.svelte';

    let {
        user = $bindable(),
        webIdData = $bindable(),
    }: {
        user: UserResponse;
        webIdData: undefined | WebIdResponse;
    } = $props();

    let t = useI18n();
    let session = useSession('account');

    let innerWidth: undefined | number = $state();
    let config: undefined | UserValuesConfig = $state();
    let providers: AuthProvidersTemplate = $state([]);
    let authProvider = $derived.by(() => {
        if (user.account_type?.startsWith('federated')) {
            return providers.filter(p => p.id === user.auth_provider_id)[0];
        }
    });

    let viewModePhone = $derived(innerWidth && innerWidth < 560);
    let viewModeWideCompact = $derived(innerWidth && innerWidth < 1000);

    // a user holding any `rauthy_admin*` role (full or delegated group admin) gets a
    // link to the Admin UI from here. This is how a group admin self-manages: it cannot manage
    // itself inside the Admin UI (it's an admin there), so it uses this dashboard instead.
    let showAdminLink = $derived(session.isAnyAdmin());

    let pamUser: undefined | PamUserResponse = $state();

    let redirectUri = useParam('redirect_uri');
    let redirectUriValidated = $state('');

    let selected = $state(t.account.navInfo);
    let tabsWide = $derived.by(() => {
        let tabs = [];
        if (pamUser) {
            tabs.push('PAM');
        }

        tabs = [...tabs, t.account.navMfa, t.account.devices, t.account.connectedApps, t.account.navEdit, t.common.password];
        if (!!webIdData) {
            tabs.push('WebID');
        }
        tabs.push(t.account.other);

        return tabs;
    });
    // Logout is not a tab anymore: on the compact / phone layouts it lives in the bottom
    // control bar next to the theme, language and Admin UI controls (see `bottomControls`).
    let tabsCompact = $derived([t.account.navInfo, ...tabsWide]);

    onMount(() => {
        fetchPamUser();

        if (useParam('v').get() === 'devices') {
            selected = t.account.devices;
        }
    });

    $effect(() => {
        if (viewModePhone || viewModeWideCompact) {
            selected = t.account.navInfo;
        } else if (pamUser) {
            selected = 'PAM';
        } else {
            selected = t.account.navMfa;
        }
    });

    $effect(() => {
        if (!config) {
            return;
        }

        if (
            (config.birthdate === 'required' && !user.user_values.birthdate) ||
            (config.gender === 'required' && !user.user_values.gender) ||
            (config.city === 'required' && !user.user_values.city) ||
            (config.country === 'required' && !user.user_values.country) ||
            (config.given_name === 'required' && !user.given_name) ||
            (config.family_name === 'required' && !user.family_name) ||
            (config.phone === 'required' && !user.user_values.phone) ||
            (config.preferred_username.preferred_username === 'required' &&
                !user.user_values.preferred_username) ||
            (config.street === 'required' && !user.user_values.street) ||
            (config.tz === 'required' && !user.user_values.tz) ||
            (config.zip === 'required' && !user.user_values.zip)
        ) {
            selected = t.account.navEdit;
        }
    });

    $effect(() => {
        let uri = redirectUri.get();
        if (uri) {
            fetchRedirectClientInfo(uri);
        }
    });

    async function fetchRedirectClientInfo(redirectUri: string) {
        let res = await fetchPost('/auth/v1/clients/validate_uri', {
            redirect_uri: redirectUri,
        });
        if (res.status === 200) {
            redirectUriValidated = normalizeClientUri(redirectUri);
        } else {
            console.error('Invalid `redirect_uri`', redirectUri);
        }
    }

    async function fetchPamUser() {
        let res = await fetchGet<PamUserResponse>('/auth/v1/pam/users/self');
        if (res.body) {
            pamUser = res.body;
        }
    }
</script>

<svelte:window bind:innerWidth />

<Template id={TPL_AUTH_PROVIDERS} bind:value={providers} />
<Template id={TPL_USER_VALUES_CONFIG} bind:value={config} />

{#snippet content(phone: boolean)}
    {#if selected === t.account.navInfo}
        <AccInfo bind:user {pamUser} {providers} {authProvider} {webIdData} viewModePhone={phone} />
    {:else if selected === 'PAM' && pamUser}
        <AccPAM bind:pamUser />
    {:else if selected === t.account.navEdit}
        <AccEdit {config} bind:user viewModePhone={phone} />
    {:else if selected === t.common.password}
        <AccPassword {user} {authProvider} viewModePhone={phone} />
    {:else if selected === t.account.navMfa}
        <AccMFA bind:user />
    {:else if selected === 'WebID'}
        {#if webIdData}
            <AccWebId bind:webIdData />
        {/if}
    {:else if selected === t.account.other}
        <AccOther {user} />
    {:else if selected === t.account.connectedApps}
        <UserClients userId={user.id} />
    {:else if selected === t.account.devices}
        {#if phone}
            <Devices userId={user.id} />
        {:else}
            <AccDevices />
        {/if}
    {/if}
{/snippet}

<div class="mx-auto flex w-full max-w-6xl flex-col gap-2 p-1 sm:gap-3 sm:p-2">
    <header class="flex flex-wrap items-center gap-2">
        {#if redirectUriValidated}
            <Button onclick={() => (window.location.href = redirectUriValidated)} invisible ariaLabel="Back">
                <span class="flex items-center gap-1">
                    <IconArrowLeft width="1.25rem" />
                    <IconHome width="1.25rem" />
                </span>
            </Button>
        {/if}
        <h1 class="min-w-0 flex-1 truncate text-2xl font-bold tracking-tight">
            {`${user.given_name || ''} ${user.family_name || ''}`}
        </h1>

        <div class="flex flex-wrap items-center gap-1">
            <DateFormatSelect />
            <ThemeSwitch />
            <div class="w-28"><LangSelector openTop borderless={false} updateBackend /></div>
            {#if showAdminLink}
                <A href="/auth/v1/admin/users" hideUnderline>
                    <span title={t.account.navBackToAdmin} class="flex items-center gap-1 text-sm">
                        <IconUserGroup width="1.2rem" />
                        <span class="hidden sm:inline">{t.account.navBackToAdmin}</span>
                    </span>
                </A>
            {/if}
            <Button level={-3} onclick={redirectToLogout}>
                <span title={t.account.navLogout} class="flex items-center gap-1">
                    <IconLogout width="1.2rem" />
                    <span class="hidden sm:inline">{t.account.navLogout}</span>
                </span>
            </Button>
        </div>
    </header>

    <MissingFields {user} />

    <div class="grid gap-3 {viewModeWideCompact ? '' : 'lg:grid-cols-[minmax(20rem,26rem)_1fr]'}">
        {#if !viewModeWideCompact}
            <Card.Root class="gap-2 self-start py-3">
                <Card.Content>
                    <AccInfo bind:user {pamUser} {webIdData} {providers} {authProvider} />
                </Card.Content>
            </Card.Root>
        {/if}

        <Card.Root class="min-w-0 gap-2 py-3">
            <Card.Content class="grid gap-3">
                <Tabs tabs={viewModeWideCompact ? tabsCompact : tabsWide} bind:selected center />
                <div class="min-w-0">
                    {@render content(!!viewModePhone)}
                </div>
            </Card.Content>
        </Card.Root>
    </div>
</div>
