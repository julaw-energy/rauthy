<script lang="ts">
    import { onMount } from 'svelte';
    import { fetchDelete, fetchGet } from '$api/fetch';
    import type { AuditUserResponse } from '$api/types/events';
    import FormSection from '$lib5/FormSection.svelte';
    import Button from '$lib5/button/Button.svelte';

    let users: AuditUserResponse[] = $state([]);
    let err = $state('');
    let busy = $state('');
    let loaded = $state(false);

    async function load() {
        const res = await fetchGet<AuditUserResponse[]>('/auth/v1/events/audit/users');
        loaded = true;
        if (res.body) users = res.body;
        else err = res.error?.message || 'The list could not be loaded';
    }

    async function wipe(u: AuditUserResponse) {
        err = '';
        busy = u.id;
        const res = await fetchDelete(`/auth/v1/events/audit/users/${encodeURIComponent(u.id)}`);
        busy = '';
        if (res.error) err = res.error.message;
        else await load();
    }

    onMount(load);

    // `deleted_at` is in seconds
    const when = (s: number) => new Date(s * 1000).toLocaleString();
    const name = (u: AuditUserResponse) => [u.given_name, u.family_name].filter(Boolean).join(' ');
</script>

<div class="flex max-w-5xl flex-col gap-4">
    <FormSection
        title="Deleted users"
        desc="When a user is deleted, their email, names, username and phone are kept so that the history of other things can still say who changed them. Wipe the personal data of a user here, for example on a deletion request: only the id stays, and the history then shows the id. This cannot be undone."
    >
        {#if err}
            <p class="text-destructive text-sm">{err}</p>
        {/if}

        {#if loaded && users.length === 0}
            <p class="text-muted-foreground text-sm">No user has been deleted since the audit log exists.</p>
        {:else}
            <div class="overflow-x-auto">
                <table class="w-full text-left text-sm">
                    <thead class="text-muted-foreground border-b text-xs uppercase">
                        <tr>
                            <th class="py-2 pr-3">Deleted</th>
                            <th class="py-2 pr-3">Email</th>
                            <th class="py-2 pr-3">Name</th>
                            <th class="py-2 pr-3">Username</th>
                            <th class="py-2 pr-3">Phone</th>
                            <th class="py-2 pr-3">Id</th>
                            <th class="py-2"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each users as u (u.id)}
                            <tr class="border-b align-top last:border-0">
                                <td class="py-2 pr-3 whitespace-nowrap">{when(u.deleted_at)}</td>
                                <td class="py-2 pr-3 break-all">{u.email ?? ''}</td>
                                <td class="py-2 pr-3">{name(u)}</td>
                                <td class="py-2 pr-3">{u.preferred_username ?? ''}</td>
                                <td class="py-2 pr-3">{u.phone ?? ''}</td>
                                <td class="text-muted-foreground py-2 pr-3 font-mono text-xs break-all">{u.id}</td>
                                <td class="py-2 text-right">
                                    {#if u.anonymized}
                                        <span class="text-muted-foreground text-xs">wiped</span>
                                    {:else}
                                        <Button level={2} onclick={() => wipe(u)} isLoading={busy === u.id}>
                                            Wipe personal data
                                        </Button>
                                    {/if}
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </FormSection>
</div>
