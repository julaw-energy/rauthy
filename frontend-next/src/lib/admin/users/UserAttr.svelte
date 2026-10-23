<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { onMount } from 'svelte';
    import Button from '$lib5/button/Button.svelte';
    import type {
        UserAttrValueRequest,
        UserAttrValuesUpdateRequest,
        UserResponse,
    } from '$api/types/user.ts';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import { fetchGet, fetchPut } from '$api/fetch';
    import type {
        UserAttrConfigResponse,
        UserAttrConfigValueResponse,
        UserAttrValueResponse,
        UserAttrValuesResponse,
    } from '$api/types/user_attrs.ts';
    import InputAttr from '$lib5/form/InputAttr.svelte';
    import { attrFromText, attrToText, validateAttr } from '$lib5/attrTypes';

    let {
        user,
        onSave,
    }: {
        user: UserResponse;
        onSave: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');
    let success = $state(false);
    // bumped after a save, so verifiable attributes reload their verification state
    let saved = $state(0);

    interface AttrValue {
        name: string;
        desc?: string;
        value: string;
        defaultValue: string | undefined;
        userEditable: boolean;
        typ?: UserAttrConfigValueResponse['typ'];
        regex?: string;
        error_msg?: string;
        options?: string[];
        default_value?: UserAttrConfigValueResponse['default_value'];
    }

    let attrsAll: UserAttrConfigValueResponse[] = $state([]);
    let attrsUser: UserAttrValueResponse[] = $state([]);

    let attrsCombined: AttrValue[] = $state([]);

    onMount(() => {
        fetchAllAttrs();
    });

    $effect(() => {
        fetchUserAttr();
    });

    $effect(() => {
        attrsCombined = attrsAll.map(a => {
            let v: AttrValue = {
                name: a.name,
                desc: a.desc,
                value: attrToText(a.typ, attrsUser.find(au => au.key === a.name)?.value),
                defaultValue: attrToText(a.typ, a.default_value),
                userEditable: a.user_editable || false,
                typ: a.typ,
                regex: a.regex,
                error_msg: a.error_msg,
                options: a.options,
                default_value: a.default_value,
            };
            return v;
        });
    });

    async function fetchAllAttrs() {
        let res = await fetchGet<UserAttrConfigResponse>('/auth/v1/users/attr');
        if (res.body) {
            attrsAll = res.body.values.toSorted((a, b) => a.name.localeCompare(b.name));
        } else {
            err = res.error?.message || 'Error fetching attrs';
        }
    }

    async function fetchUserAttr() {
        let res = await fetchGet<UserAttrValuesResponse>(`/auth/v1/users/${user.id}/attr`);
        if (res.body) {
            attrsUser = res.body.values.map(v => {
                v.value = attrToText(attrsAll.find(a => a.name === v.key)?.typ, v.value);
                return v;
            });
        } else {
            err = res.error?.message || 'Error fetching users attrs';
        }
    }

    async function onSubmit() {
        err = '';

        for (let a of attrsCombined) {
            let msg = validateAttr(a, a.value);
            if (msg) {
                err = `${a.name}: ${msg}`;
                return;
            }
        }

        let userKeys = attrsUser.map(a => a.key);
        let payload: UserAttrValuesUpdateRequest = {
            values: attrsCombined
                .filter(a => a.value.trim() || userKeys.includes(a.name))
                .map(a => {
                    let v: UserAttrValueRequest = {
                        key: a.name,
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
        }

        onSave();
    }
</script>

<div class="grid max-w-xl gap-4">
    {#if attrsCombined.length === 0}
        <p class="text-muted-foreground text-sm">{ta.common.noEntries}</p>
    {:else}
        <p class="text-muted-foreground text-sm">{ta.users.descAttr}</p>

        {#each attrsCombined as a (a.name)}
            <InputAttr def={a} bind:value={a.value} onEnter={onSubmit} userId={user.id} admin refresh={saved} />
        {/each}

        <div class="flex items-center gap-3">
            <Button onclick={onSubmit}>
                {t.common.save}
            </Button>
            <Msg {success} />
        </div>
    {/if}

    <Msg {err} />
</div>
