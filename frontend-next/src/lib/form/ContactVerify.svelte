<script lang="ts">
    import { Badge } from '$lib/components/ui/badge';
    import Button from '$lib/components/ui/button/button.svelte';
    import { Input } from '$lib/components/ui/input';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type {
        AttrVerificationResponse,
        ChannelOption,
        ContactKind,
        VerificationStatusResponse,
    } from '$api/types/verification';
    import { BadgeCheck, TriangleAlert } from '@lucide/svelte';

    // Shows whether an email address / phone number is verified, and lets the user (or an admin)
    // verify it with a code. The code goes out through the configured email / SMS channel.
    let {
        userId,
        kind,
        // the value currently in the form: a verification always belongs to the SAVED value
        current,
        // admins can additionally mark a phone number verified / revoke it without a code
        admin = false,
        // set for a custom attribute of type `verified_email` / `verified_phone`: the value is
        // then the attribute's instead of the user's own email / phone number
        attr,
        // changes whenever the saved value may have changed, to force a reload
        refresh = 0,
    }: {
        userId: string;
        kind: ContactKind;
        attr?: string;
        refresh?: number;
        current?: string | null;
        admin?: boolean;
    } = $props();

    // the same endpoints, just under the attribute for custom attributes
    const base = $derived(
        attr
            ? `/auth/v1/users/${userId}/attr/${attr}/verification`
            : `/auth/v1/users/${userId}/verification`,
    );
    const actionUrl = (action: 'start' | 'confirm' | 'set') =>
        attr
            ? action === 'set'
                ? base
                : `${base}/${action}`
            : action === 'set'
              ? `${base}/${kind}`
              : `${base}/${kind}/${action}`;

    let status: { contact?: { value: string; verified: boolean }; canSend: boolean } | undefined =
        $state();
    let contact = $derived(status?.contact);
    let canSend = $derived(!!status?.canSend);
    let dirty = $derived(!!contact && (current ?? '').trim() !== contact.value);

    let step: 'idle' | 'code' = $state('idle');
    let code = $state('');
    let busy = $state(false);
    let err = $state('');
    let info = $state('');
    let cooldown = $state(0);
    // where the code can go. The first one is the default, the user may pick another.
    let channels: ChannelOption[] = $state([]);
    let channel = $state('');

    // loads on mount and again when the saved value was changed from outside (e.g. after saving)
    $effect(() => {
        userId;
        current;
        refresh;
        if (!dirty) load();
    });

    async function load() {
        if (attr) {
            let res = await fetchGet<AttrVerificationResponse>(base);
            if (res.body) {
                status = {
                    contact: res.body.value
                        ? { value: res.body.value, verified: res.body.verified }
                        : undefined,
                    canSend: res.body.verify_available,
                };
                setChannels(res.body.channels);
            }
        } else {
            let res = await fetchGet<VerificationStatusResponse>(base);
            if (res.body) {
                status = {
                    contact: kind === 'email' ? res.body.email : res.body.phone,
                    canSend:
                        (kind === 'email' ? res.body.email_channels : res.body.phone_channels).length > 0,
                };
                setChannels(kind === 'email' ? res.body.email_channels : res.body.phone_channels);
            }
        }
    }

    function setChannels(list: ChannelOption[]) {
        channels = list;
        if (!list.some(c => c.name === channel)) channel = list[0]?.name ?? '';
    }

    async function start() {
        err = '';
        info = '';
        busy = true;
        let res = await fetchPost(
            channel ? `${actionUrl('start')}?channel=${encodeURIComponent(channel)}` : actionUrl('start'),
        );
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        step = 'code';
        info = `Code sent through ${channels.find(c => c.name === channel)?.label ?? 'the channel'}.`;
        cooldown = 60;
        const timer = setInterval(() => {
            cooldown -= 1;
            if (cooldown <= 0) clearInterval(timer);
        }, 1000);
    }

    async function confirm() {
        err = '';
        busy = true;
        let res = await fetchPost(actionUrl('confirm'), {
            code: code.trim(),
        });
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        step = 'idle';
        code = '';
        info = '';
        await load();
    }

    async function setVerified(verified: boolean) {
        err = '';
        busy = true;
        let res = await fetchPut(actionUrl('set'), { verified });
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        await load();
    }
</script>

{#if contact}
    <div class="grid gap-2 text-sm">
        <div class="flex flex-wrap items-center gap-2">
            {#if contact.verified && !dirty}
                <Badge variant="secondary" class="gap-1 text-green-600 dark:text-green-400">
                    <BadgeCheck class="size-3.5" />Verified
                </Badge>
                {#if admin && (kind === 'phone' || !!attr)}
                    <Button type="button" size="sm" variant="ghost" disabled={busy} onclick={() => setVerified(false)}>
                        Revoke
                    </Button>
                {/if}
            {:else if dirty}
                <span class="text-muted-foreground text-xs">Save your changes to verify the new value.</span>
            {:else}
                <Badge variant="outline" class="gap-1 text-amber-600 dark:text-amber-400">
                    <TriangleAlert class="size-3.5" />Not verified
                </Badge>
                {#if step === 'idle'}
                    {#if canSend}
                        {#if channels.length > 1}
                            <select
                                class="border-input bg-background h-8 rounded-md border px-2 text-xs"
                                aria-label="Send the code through"
                                bind:value={channel}
                            >
                                {#each channels as c (c.name)}
                                    <option value={c.name}>{c.label}</option>
                                {/each}
                            </select>
                        {/if}
                        <Button type="button" size="sm" variant="outline" disabled={busy} onclick={start}>
                            {channels.length === 1 ? `Verify by ${channels[0].label}` : 'Verify'}
                        </Button>
                    {:else}
                        <span class="text-muted-foreground text-xs">
                            No channel is set up that can verify this value.
                        </span>
                    {/if}
                    {#if admin && (kind === 'phone' || !!attr)}
                        <Button type="button" size="sm" variant="ghost" disabled={busy} onclick={() => setVerified(true)}>
                            Mark verified
                        </Button>
                    {/if}
                {/if}
            {/if}
        </div>

        {#if step === 'code' && !contact.verified && !dirty}
            <div class="flex flex-wrap items-center gap-2">
                <Input
                    class="h-8 w-32 font-mono tracking-widest"
                    inputmode="numeric"
                    maxlength={6}
                    placeholder="000000"
                    aria-label="Verification code"
                    bind:value={code}
                    onkeydown={(ev: KeyboardEvent) => {
                        if (ev.key === 'Enter') {
                            ev.preventDefault();
                            if (code.length === 6) confirm();
                        }
                    }}
                />
                <Button type="button" size="sm" disabled={busy || code.length !== 6} onclick={confirm}>
                    Confirm
                </Button>
                <Button type="button" size="sm" variant="ghost" disabled={busy || cooldown > 0} onclick={start}>
                    {cooldown > 0 ? `Resend in ${cooldown}s` : 'Resend'}
                </Button>
            </div>
        {/if}

        {#if info && !err}<span class="text-muted-foreground text-xs">{info}</span>{/if}
        {#if err}<span class="text-destructive text-xs">{err}</span>{/if}
    </div>
{/if}
