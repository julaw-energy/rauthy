<script lang="ts">
    import { onMount } from 'svelte';
    import Argon2Utility from '$lib5/admin/config/argon2/Argon2Utility.svelte';
    import { fetchGet } from '$api/fetch';
    import type { Argon2ParamsResponse, LoginTimeResponse } from '$api/types/login_times.ts';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';

    let ta = useI18nAdmin();

    let params: undefined | Argon2ParamsResponse = $state();
    let loginTime: undefined | number = $state();
    let numCpus: undefined | number = $state();

    onMount(async () => {
        let res = await fetchGet<LoginTimeResponse>('/auth/v1/login_time');
        if (res.body) {
            params = res.body.argon2_params;
            loginTime = res.body.login_time;
            numCpus = res.body.num_cpus;
        } else {
            console.error(res.error);
        }
    });
</script>

<div class="grid max-w-3xl gap-4">
    <h3 class="text-xl font-semibold">Argon2ID - Password Hashing</h3>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.pUtility}</p>
    <p class="text-sm">
        <b>{ta.docs.hashing.tune}</b><br />
        {ta.docs.hashing.pTune}
    </p>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.pDetails}</p>
    <div class="grid gap-3">
        <div class="bg-card rounded-xl border p-4 text-sm">
            <b>m_cost</b>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.mCost1}</p>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.mCost2}</p>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.mCost3}</p>
        </div>
        <div class="bg-card rounded-xl border p-4 text-sm">
            <b>p_cost</b>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.pCost1}</p>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.pCost2}</p>
        </div>
        <div class="bg-card rounded-xl border p-4 text-sm">
            <b>t_cost</b>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.tCost1}</p>
            <p class="text-muted-foreground mt-1">{@html ta.docs.hashing.tCost2}</p>
        </div>
    </div>

    <h3 class="text-xl font-semibold">{ta.docs.hashing.loginTimeHead}</h3>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.loginTime1}</p>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.loginTime2}</p>

    <h3 class="text-xl font-semibold">{ta.docs.hashing.currValuesHead}</h3>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.currValues1}</p>
    <div class="bg-card grid max-w-sm gap-1 rounded-xl border p-4 font-mono text-sm">
        <div class="flex justify-between"><span class="text-muted-foreground">m_cost:</span> {params?.m_cost} kB</div>
        <div class="flex justify-between"><span class="text-muted-foreground">p_cost:</span> {params?.p_cost} threads</div>
        <div class="flex justify-between"><span class="text-muted-foreground">t_cost:</span> {params?.t_cost}</div>
        <div class="flex justify-between border-t pt-1"><span class="text-muted-foreground">Login Time:</span> {loginTime} ms</div>
    </div>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.currValuesNote}</p>
    <p class="text-sm"><b>{ta.docs.hashing.currValuesThreadsAccess}</b>: {numCpus}</p>

    <h3 class="text-xl font-semibold">{ta.docs.hashing.utilityHead}</h3>
    <p class="text-muted-foreground text-sm">{ta.docs.hashing.utility1}</p>
    <p class="text-muted-foreground text-sm">{@html ta.docs.hashing.utility2}</p>

    {#if params}
        <Argon2Utility {params} />
    {/if}
</div>
