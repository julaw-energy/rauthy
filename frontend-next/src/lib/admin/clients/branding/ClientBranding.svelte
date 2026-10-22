<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import type { ClientResponse } from '$api/types/clients.ts';
    import { fetchDelete, fetchPost, fetchPut } from '$api/fetch';
    import type { ThemeRequestResponse } from '$api/types/themes.ts';
    import Form from '$lib5/form/Form.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Input from '$lib5/form/Input.svelte';
    import { PATTERN_CSS_VALUE_LOOSE } from '$utils/patterns';
    import BrandingMode from '$lib5/admin/clients/branding/BrandingMode.svelte';
    import BrandingPreviewWrapper from '$lib5/admin/clients/branding/BrandingPreviewWrapper.svelte';
    import { genKey } from '$utils/helpers';
    import InputFile from '$lib5/form/InputFile.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import IconTrash from '$icons/IconTrash.svelte';

    let {
        client,
    }: {
        client: ClientResponse;
    } = $props();

    const inputWidth = '13rem';

    let t = useI18n();
    let ta = useI18nAdmin();

    let success = $state(false);
    let err = $state('');
    let theme: ThemeRequestResponse | undefined = $state();

    let logoKey = $state(genKey());
    let logoUrl = $derived(`/auth/v1/clients/${client.id}/logo?${logoKey}`);
    let faviconKey = $state(genKey());
    let faviconLoaded = $state(false);
    let faviconUrl = $derived(`/auth/v1/clients/${client.id}/favicon?${faviconKey}`);
    let url = $derived(`/auth/v1/theme/${client.id}`);

    $effect(() => {
        fetchTheme();
    });

    async function fetchTheme() {
        let res = await fetchPost<ThemeRequestResponse>(url);
        if (res.body) {
            theme = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (!theme) {
            console.error('theme is undefined');
            return;
        }

        let payload = theme;
        // this could be `rauthy` the default has been used before
        payload.client_id = client.id;

        let res = await fetchPut(url, payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            setTimeout(() => {
                success = false;
            }, 2000);
        }
    }

    async function reset() {
        let res = await fetchDelete(url);
        if (res.error) {
            err = res.error.message;
        } else {
            await fetchDelete(`/auth/v1/clients/${client.id}/logo`);
            await fetchDelete(`/auth/v1/clients/${client.id}/favicon`);

            await fetchTheme();
            logoKey = genKey();
            faviconLoaded = false;
            faviconKey = genKey();
            success = true;
            setTimeout(() => {
                success = false;
            }, 2000);
        }
    }

    async function onUploadSuccess() {
        logoKey = genKey();
    }

    async function onLogoDelete() {
        let res = await fetchDelete(`/auth/v1/clients/${client.id}/logo`);
        if (res.error) {
            err = res.error.message;
        } else {
            logoKey = genKey();
        }
    }

    async function onFaviconDelete() {
        let res = await fetchDelete(`/auth/v1/clients/${client.id}/favicon`);
        if (res.error) {
            err = res.error.message;
        } else {
            faviconLoaded = false;
            faviconKey = genKey();
        }
    }

    function onFaviconUploadSuccess() {
        faviconLoaded = false;
        faviconKey = genKey();
    }
</script>

<Form action={url} {onSubmit}>
    {#if theme}
        <div class="grid gap-6 xl:grid-cols-[1fr_22rem]">
            <div class="grid min-w-0 content-start gap-4">
                <p class="text-muted-foreground text-sm">{@html ta.clients.branding.descVariables}</p>

                <Input
                    label="--border-radius"
                    placeholder="--border-radius"
                    errMsg={ta.validation.css}
                    width={inputWidth}
                    bind:value={theme.border_radius}
                    required
                    pattern={PATTERN_CSS_VALUE_LOOSE}
                />

                <h3 class="text-lg font-semibold">Light Theme</h3>
                <BrandingMode bind:values={theme.light} />

                <h3 class="text-lg font-semibold">Dark Theme</h3>
                <BrandingMode bind:values={theme.dark} />
            </div>

            <div class="grid min-w-0 content-start gap-4 xl:sticky xl:top-0">
                {#key logoKey}
                    <BrandingPreviewWrapper {logoUrl} {theme} />
                {/key}

                <div class="grid gap-2">
                    <p class="text-sm font-medium">Logo Upload</p>
                    <div class="flex items-center gap-2">
                        <InputFile
                            method="PUT"
                            url={`/auth/v1/clients/${client.id}/logo`}
                            fileName="logo"
                            width="100%"
                            onSuccess={onUploadSuccess}
                        />
                        <Button ariaLabel={t.common.delete} invisible level={-3} onclick={onLogoDelete}>
                            <IconTrash width="1.25rem" />
                        </Button>
                    </div>
                </div>

                <div class="grid gap-2">
                    <p class="text-sm font-medium">{ta.clients.branding.faviconUpload}</p>
                    <div class="flex items-center gap-2">
                        {#key faviconKey}
                            <img
                                class="size-8 object-contain {faviconLoaded ? '' : 'hidden'}"
                                src={faviconUrl}
                                alt={ta.clients.branding.faviconPreviewAlt}
                                onload={() => (faviconLoaded = true)}
                                onerror={() => (faviconLoaded = false)}
                            />
                        {/key}
                        <InputFile
                            method="PUT"
                            url={`/auth/v1/clients/${client.id}/favicon`}
                            fileName="favicon"
                            width="100%"
                            onSuccess={onFaviconUploadSuccess}
                        />
                        <Button ariaLabel={t.common.delete} invisible level={-3} onclick={onFaviconDelete}>
                            <IconTrash width="1.25rem" />
                        </Button>
                    </div>
                </div>

                <div class="flex flex-wrap items-center gap-2">
                    <Button type="submit">
                        {t.common.save}
                    </Button>
                    <Button level={-2} onclick={reset}>
                        {ta.common.reset}
                    </Button>
                    <Msg {success} {err} />
                </div>
            </div>
        </div>
    {/if}
</Form>
