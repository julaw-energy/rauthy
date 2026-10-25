<script lang="ts">
    import { untrack } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { ArrowDown, ArrowUp } from '@lucide/svelte';
    import { fetchGet, fetchPut } from '$api/fetch';
    import type { AttrVerifyChannelsResponse, ChannelOption } from '$api/types/verification';
    import InputCheckbox from '$lib5/form/InputCheckbox.svelte';
    import Button from '$lib5/button/Button.svelte';

    // Which channels can verify a verifiable attribute. A code goes to the attribute's value
    // through the channel the user picks; the first linked channel is the default.
    let { name, typ }: { name: string; typ: string } = $props();

    let candidates: ChannelOption[] = $state([]);
    // linked channels, in order of preference
    let linked: string[] = $state([]);
    let loaded = $state(false);
    let busy = $state(false);
    let err = $state('');

    $effect(() => {
        name;
        typ;
        untrack(load);
    });

    async function load() {
        let res = await fetchGet<AttrVerifyChannelsResponse>(`/auth/v1/users/attr/${name}/verify_channels`);
        loaded = true;
        if (res.body) {
            candidates = res.body.candidates;
            linked = res.body.channels;
        }
    }

    const label = (n: string) => candidates.find(c => c.name === n)?.label ?? n;

    function toggle(n: string, on: boolean) {
        linked = on ? [...linked, n] : linked.filter(l => l !== n);
    }

    function move(i: number, d: number) {
        let j = i + d;
        if (j < 0 || j >= linked.length) return;
        let l = [...linked];
        [l[i], l[j]] = [l[j], l[i]];
        linked = l;
    }

    async function save() {
        err = '';
        busy = true;
        let res = await fetchPut<AttrVerifyChannelsResponse>(`/auth/v1/users/attr/${name}/verify_channels`, {
            channels: linked,
        });
        busy = false;
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Verification channels saved');
        await load();
    }
</script>

{#if loaded}
    <div class="bg-muted/40 grid gap-3 rounded-xl border p-3 text-sm">
        <div class="grid gap-1">
            <span class="font-medium">Verified through</span>
            <span class="text-muted-foreground text-xs">
                The user picks one of these to receive the code. The first is the default. A channel
                appears here when it can deliver to this attribute: it is tied to the attribute itself,
                or to Email / Phone for attributes of that type.
            </span>
        </div>

        {#if candidates.length === 0}
            <p class="text-muted-foreground text-xs">
                No channel can deliver to this attribute yet. Create one under Config → Channels and
                tie it to <code>attr:{name}</code>{#if typ === 'verified_email'}, or to Email{:else if typ === 'verified_phone'}, or to Phone{/if}.
            </p>
        {:else}
            <div class="grid gap-1.5">
                {#each candidates as c (c.name)}
                    <InputCheckbox
                        bind:checked={() => linked.includes(c.name), v => toggle(c.name, v)}
                        ariaLabel={c.label}
                    >
                        {c.label}
                        <span class="text-muted-foreground font-mono text-xs">{c.name}</span>
                    </InputCheckbox>
                {/each}
            </div>

            {#if linked.length > 1}
                <div class="grid gap-1">
                    <span class="text-muted-foreground text-xs">Order of preference</span>
                    {#each linked as n, i (n)}
                        <div class="flex items-center gap-2">
                            <span class="w-5 text-xs">{i + 1}.</span>
                            <span>{label(n)}</span>
                            <button
                                type="button"
                                class="hover:bg-accent rounded p-0.5 disabled:opacity-30"
                                aria-label="Move up"
                                disabled={i === 0}
                                onclick={() => move(i, -1)}
                            >
                                <ArrowUp class="size-3.5" />
                            </button>
                            <button
                                type="button"
                                class="hover:bg-accent rounded p-0.5 disabled:opacity-30"
                                aria-label="Move down"
                                disabled={i === linked.length - 1}
                                onclick={() => move(i, 1)}
                            >
                                <ArrowDown class="size-3.5" />
                            </button>
                        </div>
                    {/each}
                </div>
            {/if}

            {#if linked.length === 0 && (typ === 'verified_email' || typ === 'verified_phone')}
                <p class="text-muted-foreground text-xs">
                    Nothing linked: {typ === 'verified_email' ? 'the Email' : 'the SMS'} channel is used.
                </p>
            {:else if linked.length === 0}
                <p class="text-destructive text-xs">Nothing linked: users cannot verify this attribute yet.</p>
            {/if}

            <div class="flex items-center gap-3">
                <Button level={2} onclick={save} isLoading={busy}>Save channels</Button>
                {#if err}<span class="text-destructive text-xs">{err}</span>{/if}
            </div>
        {/if}
    </div>
{/if}
