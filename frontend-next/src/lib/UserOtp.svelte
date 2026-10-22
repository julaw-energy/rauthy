<script lang="ts">
    import { formatDateFromTs } from '$utils/helpers';
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import type { OtpResponse } from '$api/types/otp';
    import IconCheckBadge from '$icons/IconCheckBadge.svelte';
    import IconStop from '$icons/IconStop.svelte';

    let {
        otp,
        showInactive,
        onDelete,
    }: {
        otp: OtpResponse;
        showInactive: boolean;
        onDelete: (name: string) => void;
    } = $props();

    let t = useI18n();
</script>

{#if otp.is_active || showInactive}
    <div class="bg-card grid gap-1.5 rounded-xl border p-3 text-sm">
        <div class="flex flex-wrap items-center justify-between gap-2">
            <span class="text-muted-foreground">
                {#if otp.kind == 'email'}
                    {t.mfa.otp.titleEmail}
                {:else}
                    <!-- todo: implement other kind of otp -->
                    Not implemented
                {/if}
            </span>

            <div class="flex items-center gap-1.5">
                <b>{otp.name}</b>
                {#if otp.is_active}
                    <IconCheckBadge width="1rem" color="var(--primary)" />
                {:else}
                    <IconStop width="1rem" color="var(--primary)" />
                {/if}
            </div>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2">
            <span class="text-muted-foreground">{t.mfa.lastUsed}</span>
            <span class="font-mono">{formatDateFromTs(otp.last_used)}</span>
        </div>

        <div class="flex justify-end">
            <Button level={-3} onclick={() => onDelete(otp.id.toString())}>
                {t.common.delete}
            </Button>
        </div>
    </div>
{/if}
