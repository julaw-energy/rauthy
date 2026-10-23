<script lang="ts">
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import A from '$lib5/A.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte.js';
    import { useTrigger } from '$state/callback.svelte';

    let ta = useI18nAdmin();

    let ref: undefined | HTMLAnchorElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => ref?.focus());
</script>

<ContentAdmin>
    <div class="bg-card grid max-w-2xl gap-4 rounded-xl border p-6">
        <h1 class="text-2xl font-bold tracking-tight">{ta.nav.docs}</h1>

        <p>
            {ta.docs.book}
            <A bind:ref href="https://sebadob.github.io/rauthy/" target="_blank">Rauthy Book</A>
        </p>

        <p>
            {ta.docs.openapi}
            <A href="/auth/v1/docs/" target="_blank">API reference</A>
            <br />
            <b>{ta.common.note}:</b>
            <span class="text-muted-foreground">
                The reference (Scalar) is shown when <code>server.scalar_enable</code> is set, otherwise the Swagger
                UI when <code>server.swagger_ui_enable</code> is. Access needs an admin session unless
                <code>server.swagger_ui_public</code> is set.
            </span>
        </p>

        <p>
            {ta.docs.source}:
            <A href="https://github.com/sebadob/rauthy" target="_blank">GitHub</A>
        </p>
    </div>
</ContentAdmin>
