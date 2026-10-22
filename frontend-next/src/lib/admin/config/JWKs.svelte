<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { onMount } from 'svelte';
    import Button from '$lib5/button/Button.svelte';
    import { fetchGet, fetchPost } from '$api/fetch';
    import type { JWKSCerts, JWKSPublicKeyCerts } from '$api/types/oidc.ts';
    import Expandable from '$lib5/Expandable.svelte';
    import LabeledValue from '$lib5/LabeledValue.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import IconCheck from '$icons/IconCheck.svelte';

    let ta = useI18nAdmin();

    let certs: JWKSPublicKeyCerts[] = $state([]);
    let isLoading = $state(false);
    let err = $state('');
    let success = $state(false);

    onMount(() => {
        fetchCerts();
    });

    async function fetchCerts() {
        let res = await fetchGet<JWKSCerts>('/auth/v1/oidc/certs');
        if (res.body) {
            certs = res.body.keys;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function onSubmit() {
        err = '';
        isLoading = true;

        let res = await fetchPost(`/auth/v1/oidc/rotate_jwk`);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            setTimeout(() => {
                success = false;
            }, 3000);
            await fetchCerts();
        }

        isLoading = false;
    }
</script>

<div class="grid max-w-3xl gap-4">
    <h2 class="text-xl font-semibold">Json Web Keys</h2>
    <div class="text-muted-foreground grid gap-2 text-sm">
        <p>{ta.jwks.p1}</p>
        <p>{ta.jwks.p2}</p>
        <p>{ta.jwks.p3}</p>
    </div>

    <div class="grid gap-2">
        {#each certs as jwk (jwk.kid)}
            <div class="rounded-xl border p-1">
                <Expandable>
                    {#snippet summary()}
                        <span class="font-mono break-all">
                            {jwk.alg}
                            /
                            {jwk.kid}
                        </span>
                    {/snippet}
                    {#snippet details()}
                        <div class="grid gap-3">
                            <LabeledValue label="Key ID" mono>
                                {jwk.kid}
                            </LabeledValue>
                            <div class="grid gap-3 sm:grid-cols-2">
                                <LabeledValue label={ta.jwks.type} mono>
                                    {jwk.kty}
                                </LabeledValue>
                                <LabeledValue label={ta.jwks.alg} mono>
                                    {jwk.alg}
                                </LabeledValue>
                            </div>

                            {#if jwk.crv}
                                <LabeledValue label="Curve" mono>
                                    {jwk.crv}
                                </LabeledValue>
                            {/if}
                            {#if jwk.n}
                                <LabeledValue label="n" mono>
                                    <div class="max-h-32 overflow-auto break-all whitespace-normal">
                                        {jwk.n}
                                    </div>
                                </LabeledValue>
                            {/if}
                            {#if jwk.e}
                                <LabeledValue label="e" mono>
                                    {jwk.e}
                                </LabeledValue>
                            {/if}
                            {#if jwk.x}
                                <LabeledValue label="x" mono>
                                    {jwk.x}
                                </LabeledValue>
                            {/if}
                        </div>
                    {/snippet}
                </Expandable>
            </div>
        {/each}
    </div>

    <div class="flex items-center gap-3">
        <Button onclick={onSubmit} {isLoading}>
            {ta.jwks.rotateKeys}
        </Button>
        <Msg {success} {err} />
    </div>
</div>
