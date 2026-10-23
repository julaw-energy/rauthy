<script lang="ts">
    import { LogIn, UserPlus, ShieldCheck } from '@lucide/svelte';
    import Button from '$lib5/button/Button.svelte';
    import { brand } from '$lib5/appName.svelte';
    import Main from '$lib5/Main.svelte';
    import ContentCenter from '$lib5/ContentCenter.svelte';
    import Template from '$lib5/Template.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import LangSelector from '$lib5/LangSelector.svelte';
    import RauthyLogo from '$lib5/RauthyLogo.svelte';
    import * as Card from '$lib/components/ui/card';
    import { useI18n } from '$state/i18n.svelte';
    import { TPL_ADMIN_BUTTON_HIDE, TPL_IS_REG_OPEN } from '$utils/constants';

    let t = useI18n();
    let hideAdminButton = $state(true);
    let isRegOpen = $state(false);

    function go(path: string) {
        window.location.href = path;
    }
</script>

<svelte:head>
    <title>Rauthy Home</title>
</svelte:head>

<Template id={TPL_ADMIN_BUTTON_HIDE} bind:value={hideAdminButton} />
<Template id={TPL_IS_REG_OPEN} bind:value={isRegOpen} />

<Main>
    <ContentCenter>
        <Card.Root class="w-full max-w-sm">
            <Card.Header class="items-center text-center">
                <RauthyLogo width="6rem" />
                <Card.Title class="sr-only">{brand.name}</Card.Title>
            </Card.Header>
            <Card.Content class="grid gap-2">
                <Button ariaLabel={t.index.accountLogin} onclick={() => go('/auth/v1/account')}>
                    <LogIn class="size-4" />
                    {t.index.accountLogin}
                </Button>
                {#if isRegOpen}
                    <Button ariaLabel={t.index.register} level={2} onclick={() => go('/auth/v1/users/register')}>
                        <UserPlus class="size-4" />
                        {t.index.register}
                    </Button>
                {/if}
                {#if !hideAdminButton}
                    <Button ariaLabel={t.index.adminLogin} level={3} onclick={() => go('/auth/v1/admin')}>
                        <ShieldCheck class="size-4" />
                        {t.index.adminLogin}
                    </Button>
                {/if}
            </Card.Content>
        </Card.Root>

        <ThemeSwitch absolute />
        <LangSelector absolute />
    </ContentCenter>
</Main>
