<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import { onMount } from 'svelte';
    import { redirectToLogin } from '$utils/helpers';
    import LangSelector from '$lib5/LangSelector.svelte';
    import Input from '$lib5/form/Input.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { TPL_DEVICE_USER_CODE_LENGTH } from '$utils/constants';
    import { fetchSolvePow } from '$utils/pow';
    import Main from '$lib5/Main.svelte';
    import ContentCenter from '$lib5/ContentCenter.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Template from '$lib5/Template.svelte';
    import { useParam } from '$state/param.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import type { SessionResponse } from '$api/types/session.ts';
    import { fetchGet, fetchPost } from '$api/fetch';
    import { PATTERN_ALNUM } from '$utils/patterns';
    import type { DeviceVerifyRequest, DeviceVerifyResponse } from '$api/types/device.ts';

    let t = useI18n();
    let session: undefined | SessionResponse = $state();

    let err = $state('');
    let isError = $state(false);
    let isLoading = $state(false);
    let userCodeLength = $state(8);

    let scopes: undefined | string[] = $state();
    let isAccepted = $state(false);
    let isDeclined = $state(false);

    let userCode = $state('');

    onMount(async () => {
        let param = useParam('code');
        let code = param.get() || '';
        userCode = code;

        let res = await fetchGet<SessionResponse>('/auth/v1/oidc/sessioninfo');
        if (res.body) {
            if (res.body.state != 'Auth') {
                redirectToLogin(`device?code=${code}`);
            }
            session = res.body;
        } else if (code) {
            redirectToLogin(`device?code=${code}`);
        } else {
            redirectToLogin('device');
        }
    });

    async function onSubmit(deviceAccepted: 'accept' | 'decline' | 'pending') {
        err = '';

        if (isError) {
            return;
        }
        if (userCode.length < userCodeLength) {
            err = t.common.errTooShort;
            return;
        }
        if (userCode.length > userCodeLength) {
            err = t.common.errTooLong;
            return;
        }
        isLoading = true;

        let pow = await fetchSolvePow();
        if (!pow) {
            err = 'PoW error - please contact your administrator';
            return;
        }
        let payload: DeviceVerifyRequest = {
            user_code: userCode,
            pow,
            device_accepted: deviceAccepted,
        };
        let res = await fetchPost<DeviceVerifyResponse>('/auth/v1/oidc/device/verify', payload);
        if (res.status === 200) {
            scopes = res.body?.scopes?.split(' ') || ['openid'];
        } else if (res.status === 202) {
            isAccepted = true;
            setTimeout(() => {
                window.location.replace('/auth/v1/account?v=devices');
            }, 2000);
        } else if (res.status === 204) {
            isDeclined = true;
        } else if (res.status === 404) {
            err = t.device.wrongOrExpired;
        } else {
            console.error(res);
            err = res.error?.message || '';
        }

        isLoading = false;
    }
</script>

<svelte:head>
    <title>{t?.device.title || 'Device Authorization'}</title>
</svelte:head>

<Template id={TPL_DEVICE_USER_CODE_LENGTH} bind:value={userCodeLength} />

<Main>
    <ContentCenter>
        {#if session}
            <Card.Root class="w-full max-w-sm">
                <Card.Header>
                    <Card.Title class="text-xl">{t.device.title}</Card.Title>
                </Card.Header>
                <Card.Content class="grid gap-4">
                    {#if scopes === undefined}
                        <p class="text-muted-foreground text-sm">
                            {t.device.desc.replace('{{count}}', userCodeLength.toString())}
                        </p>

                        <Input
                            name="userCode"
                            autocomplete="off"
                            label={t.device.userCode}
                            placeholder={t.device.userCode}
                            bind:value={userCode}
                            bind:isError
                            required
                            min={userCodeLength.toString()}
                            max={userCodeLength.toString()}
                            pattern={PATTERN_ALNUM}
                        />

                        <Button onclick={() => onSubmit('pending')} {isLoading} class="w-full">
                            {t.device.submit}
                        </Button>
                    {:else if isAccepted}
                        <div class="grid gap-2 text-sm">
                            <p class="text-primary font-medium">{t.device.isAccepted}</p>
                            <p class="text-muted-foreground">{t.device.autoRedirectAccount}</p>
                        </div>
                    {:else if isDeclined}
                        <div class="grid gap-2 text-sm">
                            <p class="text-destructive font-medium">{t.device.isDeclined}</p>
                            <p class="text-muted-foreground">{t.device.closeWindow}</p>
                        </div>
                    {:else}
                        <div class="grid gap-2 text-sm">
                            <p>{t.device.descScopes}</p>
                            <ul class="list-disc pl-5">
                                {#each scopes as scope (scope)}
                                    <li>{scope}</li>
                                {/each}
                            </ul>
                        </div>

                        <div class="flex flex-wrap gap-2">
                            <Button onclick={() => onSubmit('accept')} {isLoading}>
                                {t.device.accept}
                            </Button>
                            <Button level={-1} onclick={() => onSubmit('decline')} {isLoading}>
                                {t.device.decline}
                            </Button>
                        </div>
                    {/if}

                    {#if err}
                        <p class="text-destructive text-sm" role="alert">{err}</p>
                    {/if}
                </Card.Content>
            </Card.Root>
        {/if}

        <ThemeSwitch absolute />
        <LangSelector absolute />
    </ContentCenter>
</Main>
