<script lang="ts">
    import type { PasskeyResponse } from '$api/types/webauthn.ts';
    import { formatDateFromTs } from '$utils/helpers';
    import IconFingerprint from '$icons/IconFingerprint.svelte';
    import Tooltip from '$lib5/Tooltip.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import IconKey from '$icons/IconKey.svelte';
    import IconCheckBadge from '$icons/IconCheckBadge.svelte';

    let {
        passkey,
        showDelete,
        onDelete,
    }: {
        passkey: PasskeyResponse;
        showDelete: boolean;
        onDelete: (name: string) => void;
    } = $props();

    let t = useI18n();
</script>

<div class="bg-card grid gap-1.5 rounded-xl border p-3 text-sm">
    <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-muted-foreground">{t.mfa.passkeyName}</span>
        <div class="flex items-center gap-1.5">
            <b>{passkey.name}</b>
            {#if passkey.user_verified}
                <Tooltip text={t.account.userVerifiedTooltip}>
                    <IconFingerprint width="1rem" color="var(--primary)" />
                </Tooltip>
            {/if}
            {#if passkey.resident_key}
                <Tooltip text={t.account.residentKeyTooltip}>
                    <IconKey width="1rem" color="var(--primary)" />
                </Tooltip>
            {/if}
            {#if passkey.aaguid}
                <Tooltip text={`${t.account.aaguidTooltip}: ${passkey.description}`}>
                    <IconCheckBadge width="1rem" color="var(--primary)" />
                </Tooltip>
            {/if}
        </div>
    </div>
    <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-muted-foreground">{t.mfa.registered}</span>
        <span class="font-mono">{formatDateFromTs(passkey.registered)}</span>
    </div>
    <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-muted-foreground">{t.mfa.lastUsed}</span>
        <span class="font-mono">{formatDateFromTs(passkey.last_used)}</span>
    </div>

    {#if showDelete}
        <div class="flex justify-end">
            <Button level={-3} onclick={() => onDelete(passkey.name)}>
                {t.common.delete}
            </Button>
        </div>
    {/if}
</div>
