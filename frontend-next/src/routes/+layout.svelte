<script lang="ts">
    import './layout.css';
    import { onMount, type Snippet } from 'svelte';
    import { ModeWatcher } from 'mode-watcher';
    import { Toaster } from '$lib/components/ui/sonner';
    import { initLang } from '$state/language.svelte';
    import { initI18n, useI18n } from '$state/i18n.svelte';
    import { brand, brandI18n, initBranding } from '$lib5/appName.svelte';
    import themeInit from '$lib/themeInit.js?url';

    initLang();
    initI18n();
    initBranding();
    brandI18n(useI18n());

    let { children }: { children: Snippet } = $props();

    let isSecureContext = $state(true);
    // We expect true to not break SSR.
    let cookiesEnabled = $state(true);

    onMount(() => {
        cookiesEnabled = navigator.cookieEnabled;
        isSecureContext = window.crypto?.subtle !== undefined;

        // the backend serves the (per client) theme as CSS variables -> let shadcn follow them
        if (getComputedStyle(document.body).getPropertyValue('--action').trim()) {
            document.body.classList.add('rauthy-theme');
        }
    });
</script>

<!-- its inline head script would violate the CSP (script-src has no 'unsafe-inline'); the mode is applied on mount instead -->
<ModeWatcher disableHeadScriptInjection />
<svelte:head>
    <script src={themeInit}></script>
</svelte:head>
<Toaster />

<noscript>
    <h2>You need to enable JavaScript</h2>
</noscript>

{#if !isSecureContext}
    <div class="flex min-h-dvh flex-col items-center justify-center gap-2 p-4 text-center">
        <h1 class="text-2xl font-bold">Not a secure browser context</h1>
        <p class="text-muted-foreground">
            You are either not in a secure browser context, or <code>window.crypto.subtle</code>
            is not available.
        </p>
    </div>
{:else if cookiesEnabled}
    {@render children()}
{:else}
    <div class="flex min-h-dvh flex-col items-center justify-center gap-2 p-4 text-center">
        <h1 class="text-2xl font-bold">Cookies disabled</h1>
        <p class="text-muted-foreground">
            You need to enable Cookies.<br />
            Without them, a safe interaction with {brand.name} is not possible.
        </p>
    </div>
{/if}
