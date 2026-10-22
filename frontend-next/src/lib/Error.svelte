<script lang="ts">
    import { TriangleAlert } from '@lucide/svelte';
    import Template from '$lib5/Template.svelte';
    import Main from '$lib5/Main.svelte';
    import ContentCenter from '$lib5/ContentCenter.svelte';
    import ThemeSwitch from '$lib5/ThemeSwitch.svelte';
    import LangSelector from '$lib5/LangSelector.svelte';
    import Expandable from '$lib5/Expandable.svelte';
    import { TPL_ERROR_TEXT, TPL_STATUS_CODE } from '$utils/constants';
    import { useI18n } from '$state/i18n.svelte';

    let t = useI18n();

    let statusCode = $state('');
    let errorText = $state('');
    let errorDetails = $state('');
</script>

<Template id={TPL_STATUS_CODE} bind:value={statusCode} />
<Template id={TPL_ERROR_TEXT} bind:value={errorText} />

<Main>
    <ContentCenter>
        <div class="flex max-w-xl flex-col items-center gap-3 text-center">
            <TriangleAlert class="text-destructive size-10" />
            <h1 class="text-5xl font-bold tracking-tight">{statusCode}</h1>
            <p class="text-muted-foreground">{errorText}</p>

            {#if errorDetails}
                <Expandable>
                    {#snippet summary()}
                        {t.error.details}
                    {/snippet}
                    {#snippet details()}
                        {errorDetails}
                    {/snippet}
                </Expandable>
            {/if}
        </div>
        <ThemeSwitch absolute />
        <LangSelector absolute />
    </ContentCenter>
</Main>
