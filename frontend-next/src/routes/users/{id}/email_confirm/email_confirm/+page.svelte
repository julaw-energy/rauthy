<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import LangSelector from '$lib5/LangSelector.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Main from '$lib5/Main.svelte';
    import ContentCenter from '$lib5/ContentCenter.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import Template from '$lib5/Template.svelte';
    import { TPL_EMAIL_NEW, TPL_EMAIL_OLD } from '$utils/constants.js';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';

    let t = useI18n();

    let emailOld = $state('old@mail.org');
    let emailNew = $state('new@mail.org');
</script>

<svelte:head>
    <title>{t.emailChange.title || 'E-Mail Change Confirm'}</title>
</svelte:head>

<Template id={TPL_EMAIL_OLD} bind:value={emailOld} />
<Template id={TPL_EMAIL_NEW} bind:value={emailNew} />

<Main>
    <ContentCenter>
        <Card.Root class="w-full max-w-md">
            <Card.Header>
                <Card.Title class="text-xl">{t.emailChange.title}</Card.Title>
            </Card.Header>
            <Card.Content class="grid gap-4 text-sm">
                <p>
                    {t.emailChange.textChanged}:<br />
                    <b>{emailOld}</b>
                    {t.emailChange.to}
                    <b>{emailNew}</b>
                </p>
                <p class="text-muted-foreground">{t.emailChange.textLogin}</p>
                <div>
                    <Button onclick={() => window.location.replace('/auth/v1/account')}>
                        {t.authorize.login}
                    </Button>
                </div>
            </Card.Content>
        </Card.Root>

        <ThemeSwitch absolute />
        <LangSelector absolute />
    </ContentCenter>
</Main>
