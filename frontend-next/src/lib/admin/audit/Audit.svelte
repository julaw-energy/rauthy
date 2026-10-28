<script lang="ts">
    import { onMount } from 'svelte';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import type { AuditEntry } from '$api/types/events';
    import FormSection from '$lib5/FormSection.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import Msg from '$lib5/Msg.svelte';

    const PAGE = 50;

    let entries: AuditEntry[] = $state([]);
    let target = $state('');
    let err = $state('');
    let loading = $state(false);
    let more = $state(false);

    let wipeId = $state('');
    let wipeErr = $state('');
    let wipeOk = $state(false);

    async function load(older: boolean) {
        loading = true;
        err = '';
        const q = new URLSearchParams({ limit: String(PAGE) });
        if (target.trim()) q.set('target_id', target.trim());
        if (older && entries.length > 0) q.set('before', String(entries[entries.length - 1].at));
        const res = await fetchGet<AuditEntry[]>(`/auth/v1/events/audit?${q}`);
        loading = false;
        if (!res.body) {
            err = res.error?.message || 'The audit log could not be loaded';
            return;
        }
        entries = older ? [...entries, ...res.body] : res.body;
        more = res.body.length === PAGE;
    }

    async function wipe() {
        wipeErr = '';
        wipeOk = false;
        const id = wipeId.trim();
        if (!id) return;
        const res = await fetchDelete(`/auth/v1/events/audit/users/${encodeURIComponent(id)}`);
        if (res.error) {
            wipeErr = res.error.message;
        } else {
            wipeOk = true;
            wipeId = '';
            await load(false);
        }
    }

    onMount(() => load(false));

    const when = (ms: number) => new Date(ms).toLocaleString();
    const who = (e: AuditEntry) => e.actor_label || e.actor;
    const on = (e: AuditEntry) => {
        const id = e.target_label || e.target_id;
        return id ? `${e.target_type}: ${id}` : e.target_type;
    };
    const tone = (a: string) =>
        a === 'delete' ? 'text-destructive' : a === 'create' ? 'text-primary' : '';
</script>

<div class="flex max-w-6xl flex-col gap-4">
    <FormSection
        title="Audit log"
        desc="Who did what. The newest entries come first. <code>System</code> is the built-in user that owns what the background tasks do. Entries are only added, never changed."
    >
        <form
            class="flex items-end gap-2"
            onsubmit={e => {
                e.preventDefault();
                load(false);
            }}
        >
            <div class="grow">
                <Input
                    bind:value={target}
                    label="Only entries about this id"
                    placeholder="a user id, a client id, ..."
                    autocomplete="off"
                />
            </div>
            <Button type="submit" level={2}>Show</Button>
        </form>

        {#if err}
            <p class="text-destructive text-sm">{err}</p>
        {/if}

        <div class="overflow-x-auto">
            <table class="w-full text-left text-sm">
                <thead class="text-muted-foreground border-b text-xs uppercase">
                    <tr>
                        <th class="py-2 pr-3">When</th>
                        <th class="py-2 pr-3">Who</th>
                        <th class="py-2 pr-3">What</th>
                        <th class="py-2 pr-3">On</th>
                        <th class="py-2">Details</th>
                    </tr>
                </thead>
                <tbody>
                    {#each entries as e (e.id)}
                        <tr class="border-b align-top last:border-0">
                            <td class="py-2 pr-3 whitespace-nowrap">{when(e.at)}</td>
                            <td class="py-2 pr-3" title={e.actor}>{who(e)}</td>
                            <td class="py-2 pr-3 font-medium {tone(e.action)}">{e.action}</td>
                            <td class="py-2 pr-3 break-all" title={e.target_id ?? ''}>{on(e)}</td>
                            <td class="text-muted-foreground py-2 break-words">{e.details ?? ''}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
        {#if !loading && entries.length === 0 && !err}
            <p class="text-muted-foreground text-sm">Nothing recorded yet.</p>
        {/if}
        {#if more}
            <div>
                <Button level={2} onclick={() => load(true)}>Older entries</Button>
            </div>
        {/if}
    </FormSection>

    <FormSection
        title="Deleted users"
        desc="When a user is deleted, their email, names, username and phone are kept so that the log can still name them. Enter the id of a deleted user to wipe that data; only the id stays. This cannot be undone."
    >
        <form
            class="flex items-end gap-2"
            onsubmit={e => {
                e.preventDefault();
                wipe();
            }}
        >
            <div class="grow">
                <Input bind:value={wipeId} label="Id of the deleted user" autocomplete="off" />
            </div>
            <Button type="submit" level={2}>Wipe personal data</Button>
            <Msg success={wipeOk} err={wipeErr} text="Wiped" />
        </form>
    </FormSection>
</div>
