<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { fade } from 'svelte/transition';
    import { useI18n } from '$state/i18n.svelte.js';
    import type {
        UserAttrValueRequest,
        UserAttrValuesUpdateRequest,
        UserResponse,
    } from '$api/types/user.ts';
    import IconCheck from '$icons/IconCheck.svelte';
    import { fetchGet, fetchPut } from '$api/fetch';
    import type {
        UserEditableAttrResponse,
        UserEditableAttrsResponse,
    } from '$api/types/user_attrs';
    import { onMount } from 'svelte';
    import InputAttr from '$lib5/form/InputAttr.svelte';
    import { attrFromText, attrToText, validateAttr } from '$lib5/attrTypes';

    let {
        user = $bindable(),
        viewModePhone,
    }: {
        user: UserResponse;
        viewModePhone?: boolean;
    } = $props();

    let t = useI18n();

    let err = $state('');
    let success = $state(false);
    // bumped after a save, so verifiable attributes reload their verification state
    let saved = $state(0);

    let attrs: UserEditableAttrResponse[] = $state([]);

    onMount(() => {
        fetchAttrs();
    });

    async function fetchAttrs() {
        let res = await fetchGet<UserEditableAttrsResponse>(
            `/auth/v1/users/${user.id}/attr/editable`,
        );
        if (res.body) {
            attrs = res.body.values.map(v => {
                v.value = attrToText(v.typ, v.value);
                return v;
            });
        } else {
            err = res.error?.message || 'Error fetching User Attributes';
        }
    }

    async function onSubmit() {
        err = '';

        for (let a of attrs) {
            let msg = validateAttr(a, a.value);
            if (msg) {
                err = `${a.name}: ${msg}`;
                return;
            }
        }

        let userKeys = attrs.map(a => a.name);
        let payload: UserAttrValuesUpdateRequest = {
            values: attrs
                .filter(a => (a.value && a.value.trim()) || userKeys.includes(a.name))
                .map(a => {
                    let v: UserAttrValueRequest = {
                        key: a.name,
                        // `null` deletes the value; `undefined` would drop the field and the backend rejects that
                        value: attrFromText(a.typ, a.value) ?? null,
                    };
                    return v;
                }),
        };

        let res = await fetchPut(`/auth/v1/users/${user.id}/attr`, payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            saved++;
            setTimeout(() => {
                success = false;
            }, 3000);

            await fetchAttrs();
        }
    }
</script>

<div class="grid max-w-xl gap-4">
    {#each attrs as a (a.name)}
        <InputAttr def={a} bind:value={a.value} onEnter={onSubmit} userId={user.id} refresh={saved} />
    {/each}

    {#if attrs.length > 0}
        <div class="flex items-center gap-3">
            <Button onclick={onSubmit}>
                {t.common.save}
            </Button>
            <Msg {success} {err} />
        </div>
    {:else}
        <p class="text-muted-foreground text-sm">-</p>
    {/if}
</div>
