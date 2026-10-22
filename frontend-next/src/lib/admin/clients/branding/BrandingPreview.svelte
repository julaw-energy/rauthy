<script lang="ts">
    import type { ThemeCss } from '$api/types/themes.ts';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import RauthyLogo from '$lib/RauthyLogo.svelte';

    let {
        logoUrl,
        borderRadius,
        theme,
        typ,
    }: {
        logoUrl: string;
        borderRadius: string;
        theme: ThemeCss;
        typ: 'light' | 'dark';
    } = $props();

    const hslVar = (v: number[]) => `${v[0]} ${v[1]}% ${v[2]}%`;

    let isLoading = $state(false);
    let showDefault = $state(false);

    $effect(() => {
        if (logoUrl) {
            showDefault = false;
        }
    });

    $effect(() => {
        if (isLoading) {
            setTimeout(() => {
                isLoading = false;
            }, 2000);
        }
    });
</script>

<!-- Scoped copy of the backend theme -> shows exactly how branded pages will look -->
<div
    class="rauthy-theme-scope bg-background text-foreground overflow-hidden rounded-xl border {typ === 'dark' ? 'dark' : ''}"
    aria-label="Preview: All components inside are only for theme and colors preview and have no effect or interaction"
    style:--border-radius={borderRadius}
    style:--radius={borderRadius}
    style:--text={hslVar(theme.text)}
    style:--text-high={hslVar(theme.text_high)}
    style:--bg={hslVar(theme.bg)}
    style:--bg-high={hslVar(theme.bg_high)}
    style:--action={hslVar(theme.action)}
    style:--accent={hslVar(theme.accent)}
    style:--error={hslVar(theme.error)}
    style:--btn-text={theme.btn_text}
    style:--theme-sun={theme.theme_sun}
    style:--theme-moon={theme.theme_moon}
>
    <div class="grid justify-items-center gap-3 p-6 text-center">
        <div class="size-20">
            {#if showDefault}
                <RauthyLogo width="100%" show={typ} />
            {:else}
                <img src={logoUrl} alt="Client Logo" class="h-full w-full object-contain" onerror={() => (showDefault = true)} />
            {/if}
        </div>

        <h3 class="text-lg font-semibold">Header</h3>
        <p class="text-muted-foreground text-sm">Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam</p>
        <p class="text-sm"><span class="text-primary">--accent-color</span></p>

        <div class="w-full max-w-xs">
            <Input label="Preview" placeholder="Preview" />
        </div>

        <div class="flex flex-wrap justify-center gap-4">
            <div class="grid gap-2">
                <Button level={1} {isLoading} onclick={() => (isLoading = true)}>Button</Button>
                <Button level={2} {isLoading} onclick={() => (isLoading = true)}>Button</Button>
                <Button level={3} {isLoading} onclick={() => (isLoading = true)}>Button</Button>
            </div>
            <div class="grid gap-2">
                <Button level={-1} {isLoading} onclick={() => (isLoading = true)}>Button</Button>
                <Button level={-2} {isLoading} onclick={() => (isLoading = true)}>Button</Button>
                <Button level={-3} {isLoading} onclick={() => (isLoading = true)}>Button</Button>
            </div>
        </div>

        <a class="text-primary underline" href={window.location.href} onclick={ev => ev.preventDefault()}>Link</a>
    </div>
</div>
