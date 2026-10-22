<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import Button from '$lib/button/Button.svelte';
    import { IS_DEV, TPL_CLIENT_LOGO_UPDATED } from '$utils/constants';
    import Input from '$lib/form/Input.svelte';
    import LangSelector from '$lib/LangSelector.svelte';
    import Main from '$lib/Main.svelte';
    import ContentCenter from '$lib/ContentCenter.svelte';
    import { useI18n } from '$state/i18n.svelte.js';
    import Template from '$lib/Template.svelte';
    import { useParam } from '$state/param.svelte.js';
    import ThemeSwitch from '$lib/ThemeSwitch.svelte';
    import Form from '$lib/form/Form.svelte';
    import { fetchPost } from '$api/fetch';
    import ClientLogo from '$lib/ClientLogo.svelte';
    import { fetchSolvePow } from '$utils/pow';
    import type { RequestResetRequest } from '$api/types/authorize';

    let t = useI18n();

    let email_hint = useParam('email_hint');

    let clientLogoUpdated = $state(-1);
    let isLoading = $state(false);
    let err = $state('');
    let success = $state(false);

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        isLoading = true;

        let email = params.get('email');
        if (!email) {
            console.error('email is empty');
            return;
        }
        let pow = (await fetchSolvePow()) || '';
        let payload: RequestResetRequest = { email, pow };

        let res = await fetchPost(form.action, payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
        }

        isLoading = false;
    }
</script>

<svelte:head>
    <title>{t?.register.register || 'Register'}</title>
</svelte:head>

<Template id={TPL_CLIENT_LOGO_UPDATED} bind:value={clientLogoUpdated} />

<Main>
    <ContentCenter>
        <Card.Root class="w-full max-w-sm">
            <Card.Content class="grid gap-5">
                <div class="flex justify-center">
                    <ClientLogo
                        clientId="rauthy"
                        updated={clientLogoUpdated > -1 ? clientLogoUpdated : undefined}
                    />
                </div>

                <div class="grid gap-1 text-center">
                    <h1 class="text-xl font-semibold">{t.passwordReset.passwordReset}</h1>
                    <p class="text-muted-foreground text-sm">{t.authorize.passwordResetDesc}</p>
                </div>

                <Form action="/auth/v1/users/request_reset" {onSubmit} withPowAs="pow">
                    <Input
                        typ="email"
                        name="email"
                        autocomplete="email"
                        label={t.common.email}
                        placeholder={t.common.email}
                        required
                        value={email_hint.get()}
                    />

                    <Button type="submit" {isLoading} class="w-full">{t.authorize.passwordRequest}</Button>

                    {#if success}
                        <p class="text-primary text-center text-sm" role="status">
                            {t.authorize.passwordResetSuccess}
                        </p>
                    {:else if err}
                        <p class="text-destructive text-center text-sm" role="alert">
                            {err}
                        </p>
                    {/if}
                </Form>
            </Card.Content>
        </Card.Root>

        <ThemeSwitch absolute />
        <LangSelector absolute />
    </ContentCenter>
</Main>
