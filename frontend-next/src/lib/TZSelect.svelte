<script lang="ts">
    import Options from '$lib/Options.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { onMount } from 'svelte';
    import { fetchTimezones } from '$utils/helpers';

    let {
        value = $bindable('UTC'),
    }: {
        value?: string;
    } = $props();

    let t = useI18n();
    let options: string[] = $state(['UTC']);

    onMount(() => {
        if (value && value !== 'UTC') {
            options.push(value);
        }
        fetchTimezones().then(tz => (options = tz));
    });
</script>

<div class="grid w-full gap-1.5">
    <div class="text-sm font-medium">
        {t.common.timezone}
    </div>
    <Options ariaLabel={t.common.timezone} {options} bind:value withSearch />
</div>
