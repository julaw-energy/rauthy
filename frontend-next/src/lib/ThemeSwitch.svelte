<script lang="ts">
    import { Sun, Moon } from '@lucide/svelte';
    import { mode, toggleMode } from 'mode-watcher';
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useTheme } from '$state/theme.svelte';

    let { absolute }: { absolute?: boolean } = $props();

    let t = useI18n();
    let theme = useTheme();
    let activeTheme = $derived(mode.current === 'dark' ? 'dark' : 'light');

    // keep the shared (old UI) theme state + the backend-served theme classes in sync
    $effect(() => {
        const dark = mode.current === 'dark';
        theme.setIsDark(dark);
        document.body.classList.toggle('theme-dark', dark);
        document.body.classList.toggle('theme-light', !dark);
    });
</script>

<div class={absolute ? 'absolute top-2 left-2 z-10' : ''}>
    <div aria-live="polite" class="sr-only">{t.common.activeTheme}: {activeTheme}</div>
    <Button ariaLabel={t.common.changeTheme.replaceAll('{{ CURRENT }}', activeTheme)} invisible onclick={toggleMode}>
        {#if activeTheme === 'dark'}<Moon class="size-5" />{:else}<Sun class="size-5" />{/if}
    </Button>
</div>
