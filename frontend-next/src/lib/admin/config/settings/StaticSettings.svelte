<script lang="ts">
    import { brand } from '$lib5/appName.svelte';
    import { onMount } from 'svelte';
    import { fetchGet } from '$api/fetch';
    import type { StaticSettingsResponse } from '$api/types/settings';
    import FormSection from '$lib5/FormSection.svelte';
    import Msg from '$lib5/Msg.svelte';
    import { Badge } from '$lib/components/ui/badge';
    import InfoTip from '$lib/InfoTip.svelte';
    import { STATIC_SETTING_INFO } from './staticSettingsInfo';

    let data: StaticSettingsResponse | undefined = $state();
    let err = $state('');

    onMount(async () => {
        let res = await fetchGet<StaticSettingsResponse>('/auth/v1/settings/static');
        if (res.body) {
            data = res.body;
        } else if (res.error) {
            err = res.error.message;
        }
    });
</script>

<div class="grid max-w-3xl gap-4">
    <div>
        <h2 class="text-xl font-semibold tracking-tight">Static settings</h2>
        <p class="text-muted-foreground text-sm">
            These come from the config file and the environment. They are read when {brand.name} starts and cannot be
            changed here: edit the config, then restart. Secrets are never shown, only whether they are set.
        </p>
    </div>

    <Msg {err} />

    {#each data?.sections ?? [] as section (section.title)}
        <FormSection title={section.title}>
            <dl class="grid gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[minmax(12rem,40%)_1fr]">
                {#each section.entries as entry (entry.key)}
                    <dt class="text-muted-foreground flex items-start gap-1 font-mono text-xs break-all sm:pt-0.5">
                        <span class="min-w-0">{entry.key}</span>
                        {#if STATIC_SETTING_INFO[entry.key]}<InfoTip text={STATIC_SETTING_INFO[entry.key]} label={`About ${entry.key}`} />{/if}
                    </dt>
                    <dd class="min-w-0 break-words">
                        {#if entry.secret}
                            <Badge variant={entry.value === 'set' ? 'secondary' : 'outline'}>
                                {entry.value}
                            </Badge>
                        {:else}
                            <span class="font-mono text-xs">{entry.value}</span>
                        {/if}
                    </dd>
                {/each}
            </dl>
        </FormSection>
    {/each}
</div>
