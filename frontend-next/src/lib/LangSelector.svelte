<script lang="ts">
    import { untrack } from 'svelte';
    import { page } from '$app/state';
    import { useI18n } from '$state/i18n.svelte';
    import { getCookie } from '$utils/helpers';
    import HeaderPicker from '$lib/HeaderPicker.svelte';
    import { Globe } from '@lucide/svelte';
    import { fetchPost } from '$api/fetch';
    import type { Language } from '$api/types/i18n.ts';
    import { useI18nConfig } from '$state/i18n_config.svelte';

    let {
        absolute,
        updateBackend,
        borderless = true,
    }: {
        absolute?: boolean;
        openTop?: boolean;
        updateBackend?: boolean;
        borderless?: boolean;
    } = $props();

    const cookieAttrs = ';Path=/;SameSite=Lax;Max-Age=157680000';

    let t = useI18n();
    let initial = untrack(() => t.lang);
    let selected = $state(initial);
    let languages: undefined | Language[] = $derived(
        page.route.id?.includes('/admin') ? useI18nConfig().admin() : useI18nConfig().common(),
    );

    $effect(() => {
        let cookie = getCookie('locale');
        let current = untrack(() => selected);
        if (cookie && cookie !== current) {
            document.cookie = 'locale=' + current + cookieAttrs;
        }
    });

    $effect(() => {
        if (selected !== initial) {
            switchLang();
        }
    });

    async function switchLang() {
        // locale is technically mandatory by EU cookie laws -> we can just save without consent
        document.cookie = 'locale=' + selected.toLowerCase() + cookieAttrs;
        if (updateBackend) {
            let res = await fetchPost('/auth/v1/update_language');
            if (res.error) {
                console.error(res.error);
                return;
            }
        }
        window.location.reload();
    }
</script>

{#if languages}
    <div class={absolute ? 'absolute top-2 right-2 z-10' : ''}>
        <HeaderPicker
            icon={Globe}
            ariaLabel={t.common.selectI18n}
            options={languages.map((l) => ({ value: l, label: l }))}
            value={selected}
            onChange={(v) => (selected = v as typeof selected)}
            label={selected.toUpperCase()}
            class={borderless ? 'border-transparent bg-transparent shadow-none' : ''}
        />
    </div>
{/if}
