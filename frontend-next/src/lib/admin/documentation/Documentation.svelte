<script lang="ts">
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte.js';
    import { useTrigger } from '$state/callback.svelte';
    import { BookOpen, Braces, ExternalLink } from '@lucide/svelte';

    let ta = useI18nAdmin();

    let ref: undefined | HTMLAnchorElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => ref?.focus());

    // Where the readable docs live. Rauthy does not serve them itself yet.
    const DOCS_URL = 'https://sebadob.github.io/rauthy/';
    const API_URL = '/auth/v1/docs/';
</script>

<ContentAdmin>
    <div class="grid max-w-2xl gap-4">
        <h1 class="text-2xl font-bold tracking-tight">{ta.nav.docs}</h1>

        <div class="grid gap-3 sm:grid-cols-2">
            <a
                bind:this={ref}
                href={API_URL}
                target="_blank"
                rel="noreferrer"
                class="bg-card hover:bg-accent grid gap-2 rounded-xl border p-5 transition-colors"
            >
                <Braces class="text-primary size-6" />
                <span class="flex items-center gap-1.5 font-semibold">
                    API reference <ExternalLink class="text-muted-foreground size-3.5" />
                </span>
                <span class="text-muted-foreground text-sm">
                    Every endpoint with its request and response, and a client to try them with your current login.
                </span>
            </a>

            <a
                href={DOCS_URL}
                target="_blank"
                rel="noreferrer"
                class="bg-card hover:bg-accent grid gap-2 rounded-xl border p-5 transition-colors"
            >
                <BookOpen class="text-primary size-6" />
                <span class="flex items-center gap-1.5 font-semibold">
                    Documentation <ExternalLink class="text-muted-foreground size-3.5" />
                </span>
                <span class="text-muted-foreground text-sm">
                    Setup, configuration, authentication providers and guides.
                </span>
            </a>
        </div>
    </div>
</ContentAdmin>
