<script lang="ts">
    import LabeledValue from '$lib/LabeledValue.svelte';
    import Options from '$lib/Options.svelte';
    import Template from '$lib5/Template.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { TPL_PASSKEY_REG_TYPE } from '$utils/constants';

    let {
        residentKey = $bindable(false),
    }: {
        residentKey: boolean;
    } = $props();

    let t = useI18n();

    // `webauthn.passkey_reg_type`: space-separated, the first one is the default
    let regTypes = $state('default resident_key');
    // [Default, Resident Key] -> the index matches `t.account.passkeys.types`
    let allowed = $derived.by(() => {
        let idx = regTypes
            .split(' ')
            .filter(typ => typ === 'default' || typ === 'resident_key')
            .map(typ => (typ === 'resident_key' ? 1 : 0));
        return idx.length > 0 ? idx : [0, 1];
    });
    let options = $derived(allowed.map(i => t.account.passkeys.types[i]));
    let selected = $state(t.account.passkeys.types[0]);

    $effect(() => {
        // the Template is resolved after mount -> start with the configured default
        selected = options[0];
    });

    $effect(() => {
        residentKey = selected === t.account.passkeys.types[1];
    });
</script>

<Template id={TPL_PASSKEY_REG_TYPE} bind:value={regTypes} />

<LabeledValue label={t.account.passkeys.type}>
    {#if options.length > 1}
        <Options {options} bind:value={selected} ariaLabel={t.account.passkeys.type} />
    {:else}
        {selected}
    {/if}
</LabeledValue>
<div class="text-muted-foreground grid gap-1 text-sm">
    <p>{t.account.passkeys.typesDesc[residentKey ? 1 : 0]}</p>
    {#if residentKey}
        <p class="text-destructive">{t.account.passkeys.rkWarning}</p>
    {/if}
</div>
