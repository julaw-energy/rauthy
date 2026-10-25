<script lang="ts">
    import { fetchGet } from '$api/fetch';
    import type { UserResponse } from '$api/types/user.ts';
    import Button from '$lib/button/Button.svelte';
    import { onMount } from 'svelte';

    // Shown after a login was stopped because the client needs data this account does not have.
    // `?complete_for=<client_id>` is set by the login page, `?return=` points back at it.
    let { user }: { user: UserResponse } = $props();

    const LABELS: Record<string, string> = {
        given_name: 'Given name',
        family_name: 'Family name',
        preferred_username: 'Username',
        email_verified: 'Verified email address',
        birthdate: 'Birthdate',
        gender: 'Gender',
        tz: 'Time zone',
        phone: 'Phone number',
        street: 'Street',
        zip: 'ZIP code',
        city: 'City',
        country: 'Country',
    };

    let clientId = $state('');
    let clientName = $state('');
    let back = $state('');
    let missing: string[] = $state([]);
    let checked = $state(false);

    // reads again after the user saved something, `user` changes with every save
    $effect(() => {
        void user.user_values;
        void user.given_name;
        void user.family_name;
        if (clientId) {
            load();
        }
    });

    onMount(() => {
        let p = new URLSearchParams(window.location.search);
        clientId = p.get('complete_for') || '';
        clientName = p.get('client_name') || '';
        let r = p.get('return') || '';
        // only ever go back to the login page of this server
        back = r.startsWith('/auth/v1/oidc/authorize') ? r : '';
    });

    async function load() {
        let res = await fetchGet<{ fields: string[] }>(
            `/auth/v1/users/${user.id}/missing_fields/${encodeURIComponent(clientId)}`,
        );
        if (res.body) {
            missing = res.body.fields;
            checked = true;
        }
    }
</script>

{#if clientId && checked}
    <div class="grid gap-2 rounded-xl border p-4 {missing.length ? 'border-amber-500/60 bg-amber-500/10' : 'border-emerald-500/60 bg-emerald-500/10'}">
        {#if missing.length}
            <p class="font-medium">
                {clientName || 'The application you are signing in to'} needs some more details before
                you can continue:
            </p>
            <ul class="list-disc pl-5 text-sm">
                {#each missing as m (m)}
                    <li>{LABELS[m] ?? m.replace(/^attr:/, '')}</li>
                {/each}
            </ul>
            <p class="text-muted-foreground text-sm">
                Add what you can in the <b>Edit</b> and <b>Other</b> tabs. Anything you cannot edit
                yourself is set by an administrator, please ask them.
            </p>
        {:else}
            <p class="font-medium">All set. You can continue to {clientName || 'the application'}.</p>
            {#if back}
                <div><Button onclick={() => window.location.replace(back)}>Continue</Button></div>
            {/if}
        {/if}
    </div>
{/if}
