<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { useI18n } from '$state/i18n.svelte.js';
    import { fetchGet, fetchPost } from '$api/fetch';
    import type {
        PamHostAccessResponse,
        PamPasswordResponse,
        PamUserResponse,
    } from '$api/types/pam';
    import LabeledValue from '$lib/LabeledValue.svelte';
    import InputPassword from '$lib/form/InputPassword.svelte';
    import { slide } from 'svelte/transition';
    import { onMount } from 'svelte';
    import SearchBar from '$lib/search_bar/SearchBar.svelte';
    import PamAuthorizedKeys from '$lib/PamAuthorizedKeys.svelte';

    let {
        pamUser = $bindable(),
    }: {
        pamUser: PamUserResponse;
    } = $props();

    let t = useI18n();

    let err = $state('');

    let hosts: PamHostAccessResponse[] = $state([]);
    let hostsFiltered: PamHostAccessResponse[] = $state([]);
    let search = $state('');

    let password = $state('');
    let expTs: undefined | number = $state();
    let expSecs: undefined | number = $state();
    let interval: undefined | number;

    onMount(() => {
        fetchHosts();
    });

    $effect(() => {
        if (search.length > 0) {
            let lower = search.toLowerCase();
            hostsFiltered = hosts.filter(h => {
                if (h.hostname.toLowerCase().includes(lower)) {
                    return true;
                }
                for (let ip of h.ips) {
                    if (ip.includes(lower)) {
                        return true;
                    }
                }
                for (let alias of h.aliases) {
                    if (alias.toLowerCase().includes(lower)) {
                        return true;
                    }
                }
                return false;
            });
        } else {
            hostsFiltered = hosts;
        }
    });

    function calcExpSecs() {
        let secs = 0;
        if (expTs) {
            let ts = new Date().getTime() / 1000;
            secs = Math.floor(expTs - ts);
        }
        if (secs > 0) {
            expSecs = secs;
        } else {
            expSecs = undefined;
            expTs = undefined;
            password = '';
            clearInterval(interval);
            interval = undefined;
        }
    }

    async function fetchHosts() {
        let res = await fetchGet<PamHostAccessResponse[]>('/auth/v1/pam/hosts/user_access');
        if (res.body) {
            res.body.sort((a, b) => a.hostname.localeCompare(b.hostname));
            hosts = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchPamUser() {
        let res = await fetchGet<PamUserResponse>('/auth/v1/pam/users/self');
        if (res.body) {
            pamUser = res.body;
        }
    }

    async function generateNew() {
        let res = await fetchPost<PamPasswordResponse>('/auth/v1/pam/password');
        if (res.body) {
            password = res.body.password;
            expTs = res.body.exp;
            calcExpSecs();

            if (interval) {
                clearInterval(interval);
            }
            interval = window.setInterval(() => {
                calcExpSecs();
            }, 1000);
        } else {
            err = res.error?.message || 'Error';
        }
    }
</script>

<div class="grid max-w-3xl gap-4">
    <div class="grid gap-4 sm:grid-cols-2">
        <LabeledValue label={t.account.pam.username}>
            {pamUser.name}
        </LabeledValue>
        <LabeledValue label="Shell" mono>
            {pamUser.shell}
        </LabeledValue>
        <LabeledValue label="UID">
            {pamUser.id}
        </LabeledValue>
        <LabeledValue label="GID">
            {pamUser.gid}
        </LabeledValue>
    </div>

    {#if password}
        <div transition:slide={{ duration: 150 }}>
            <InputPassword
                label={t.common.password}
                placeholder={t.common.password}
                value={password}
                disabled
                showCopy
            />
        </div>
    {/if}

    {#if expSecs}
        <p class="text-muted-foreground text-sm" transition:slide={{ duration: 150 }}>
            {t.account.pam.validFor.replace('{{ secs }}', expSecs.toString())}
        </p>
    {/if}

    {#if expSecs === undefined || expSecs < 30}
        <div class="flex flex-wrap items-center gap-3" transition:slide={{ duration: 150 }}>
            <Button ariaLabel={t.account.pam.generatePassword} onclick={generateNew}>
                {t.account.pam.generatePassword}
            </Button>
            <Msg {err} />
        </div>
    {/if}

    <PamAuthorizedKeys authorizedKeys={pamUser.authorized_keys} onSave={fetchPamUser} />

    {#snippet btnClip(value: string)}
        <Button invisible onclick={() => navigator.clipboard.writeText(value)}>
            <span title={value} class="font-mono text-xs">{value}</span>
        </Button>
    {/snippet}

    {#snippet pamHost(host: PamHostAccessResponse)}
        <div class="bg-card grid gap-3 rounded-xl border p-4">
            <h4 class="font-semibold">{host.hostname}</h4>
            <LabeledValue label="IPs">
                <div class="flex flex-wrap">
                    {#each host.ips as ip (ip)}
                        {@render btnClip(ip)}
                    {/each}
                </div>
            </LabeledValue>
            <LabeledValue label="Aliases">
                <div class="flex flex-wrap">
                    {#each host.aliases as alias (alias)}
                        {@render btnClip(alias)}
                    {/each}
                </div>
            </LabeledValue>

            {#if host.notes}
                <p class="text-muted-foreground border-t pt-2 text-sm whitespace-pre-wrap">{host.notes}</p>
            {/if}
        </div>
    {/snippet}

    {#if hosts.length > 0}
        <div class="grid gap-3">
            <h3 class="text-lg font-semibold">Hosts</h3>

            <SearchBar bind:value={search} />

            {#each hostsFiltered as host (host.hostname)}
                {@render pamHost(host)}
            {/each}
        </div>
    {/if}
</div>
