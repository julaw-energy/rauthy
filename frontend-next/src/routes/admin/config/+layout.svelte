<script lang="ts">
    import type { Snippet } from 'svelte';
    import NavSub from '$lib5/nav/NavSub.svelte';
    import NavLinkSub from '$lib5/nav/NavLinkSub.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useTrigger } from '$state/callback.svelte';

    let { children }: { children: Snippet } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let refA: undefined | HTMLAnchorElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refA?.focus());
</script>

<svelte:head>
    <title>Rauthy Config</title>
</svelte:head>

<div class="flex flex-col gap-4 md:flex-row">
    <NavSub>
        <NavLinkSub bind:ref={refA} href="/auth/v1/admin/config/general">General</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/policy">
            {t.passwordPolicy.passwordPolicy}
        </NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/jwks">JWKS</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/argon2">Password Hashing</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/email">Email (SMTP)</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/channels">Channels</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/encryption">{ta.docs.encryption}</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/backups">Backups</NavLinkSub>
        <NavLinkSub href="/auth/v1/admin/config/tos">{ta.tos.tos}</NavLinkSub>
    </NavSub>

    <div class="min-w-0 flex-1">
        <ContentAdmin>
            {@render children()}
        </ContentAdmin>
    </div>
</div>
