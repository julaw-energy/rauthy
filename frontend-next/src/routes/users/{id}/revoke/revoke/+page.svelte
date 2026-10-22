<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import LangSelector from '$lib5/LangSelector.svelte';
    import Main from '$lib5/Main.svelte';
    import ContentCenter from '$lib5/ContentCenter.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import A from '$lib/A.svelte';

    let t = useI18n();
</script>

<svelte:head>
    <title>{t.userRevoke.title || 'Login Revoke'}</title>
</svelte:head>

<Main>
    <ContentCenter>
        <Card.Root class="w-full max-w-md">
            <Card.Header>
                <Card.Title class="text-xl">{t.userRevoke.title}</Card.Title>
            </Card.Header>
            <Card.Content class="grid gap-3 text-sm">
                <p>{t.userRevoke.desc1}</p>
                <p class="text-destructive">{t.userRevoke.desc2}</p>
                <A href="/auth/v1/account">Account</A>
            </Card.Content>
        </Card.Root>

        <ThemeSwitch absolute />
        <LangSelector absolute />
    </ContentCenter>
</Main>
