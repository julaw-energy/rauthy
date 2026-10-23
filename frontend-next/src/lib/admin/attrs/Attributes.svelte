<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import { useTrigger } from '$state/callback.svelte';
    import type { UserAttrConfigResponse, UserAttrConfigValueResponse } from '$api/types/user_attrs.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { ATTR_TYPES, attrFromText, attrToText, validateAttr, type AttrTyp } from '$lib5/attrTypes';
    import { PATTERN_ATTR } from '$utils/patterns';
    import type { EditConfig } from '$lib/components/data-table/data-table.svelte';
    import { fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import AttrAddNew from '$lib5/admin/attrs/AttrAddNew.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);
    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let attrs: UserAttrConfigValueResponse[] = $state([]);

    type Attr = UserAttrConfigValueResponse;
    const optText = (x: Attr) => (x.options ?? []).join(', ');
    const defText = (x: Attr) => attrToText(x.typ, x.default_value);
    const typeItems = [{ value: '', label: '—' }, ...ATTR_TYPES];

    const columns: FeaturedColumnDef<Attr>[] = [
        textCol<Attr>('name', ta.common.name, { size: 200 }),
        fnTextCol<Attr>('typ', 'Type', x => ATTR_TYPES.find(t => t.value === x.typ)?.label ?? '', { size: 170 }),
        textCol<Attr>('desc', ta.attrs.desc, { size: 240 }),
        fnTextCol<Attr>('default_value', ta.attrs.defaultValue, defText, { size: 200 }),
        fnTextCol<Attr>('user_editable', ta.attrs.userEditable, x => (x.user_editable ? '✓' : '–'), { size: 130 }),
        textCol<Attr>('regex', 'Pattern', { size: 200 }),
        textCol<Attr>('error_msg', 'Error message', { size: 220 }),
        fnTextCol<Attr>('options', 'Options', optText, { size: 200 }),
    ];

    const parseOptions = (text: string) =>
        text
            .split(',')
            .map(o => o.trim())
            .filter(Boolean);

    const editable: EditConfig<Attr> = {
        columns: {
            name: { required: true, maxLength: 32, pattern: PATTERN_ATTR },
            typ: { kind: 'select', items: typeItems, read: x => x.typ ?? '' },
            desc: { maxLength: 128, read: x => x.desc ?? '' },
            default_value: { read: defText },
            user_editable: { kind: 'bool', read: x => String(!!x.user_editable) },
            regex: {
                maxLength: 256,
                read: x => x.regex ?? '',
                validate: v => {
                    try {
                        new RegExp(v);
                        return null;
                    } catch {
                        return 'Invalid pattern';
                    }
                },
            },
            error_msg: { maxLength: 128, read: x => x.error_msg ?? '' },
            options: { read: optText },
        },
        save: async (x, ch) => {
            let name = ch.name ?? x.name;
            if (attrs.some(o => o.name !== x.name && o.name === name)) {
                throw new Error(ta.common.nameExistsAlready);
            }
            let typ = ((ch.typ ?? x.typ ?? '') || undefined) as AttrTyp | undefined;
            let options = parseOptions(ch.options ?? optText(x));
            let regex = ch.regex ?? x.regex ?? '';
            let def = ch.default_value ?? defText(x);
            if (typ === 'enum' && options.length === 0) {
                throw new Error('A list type needs at least one option');
            }
            let defErr = validateAttr({ name, typ, regex, options }, def);
            if (defErr) {
                throw new Error(`${ta.attrs.defaultValue}: ${defErr}`);
            }
            let res = await fetchPut(`/auth/v1/users/attr/${x.name}`, {
                name,
                desc: (ch.desc ?? x.desc) || undefined,
                default_value: attrFromText(typ, def),
                user_editable: (ch.user_editable ?? String(!!x.user_editable)) === 'true',
                typ,
                regex: regex || undefined,
                error_msg: (ch.error_msg ?? x.error_msg) || undefined,
                options: typ === 'enum' ? options : undefined,
            });
            if (res.error) throw new Error(res.error.message);
        },
        onSaved: fetchAttrs,
    };

    const remove: RowDeletion<UserAttrConfigValueResponse> = {
        label: x => x.name,
        canRemove: x => true,
        run: async x => {
            let res = await fetchDelete(`/auth/v1/users/attr/${x.name}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchAttrs,
    };

    onMount(() => {
        fetchAttrs();
    });

    async function fetchAttrs() {
        let res = await fetchGet<UserAttrConfigResponse>('/auth/v1/users/attr');
        if (res.body) {
            attrs = res.body.values;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchAttrs();
    }

    async function onAddNew(name: string, another?: boolean) {
        if (another) {
            addNonce++;
            toast.success('Saved');
            await fetchAttrs();
            return;
        }
        closeModal?.();
        await fetchAttrs();
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="attrs" class="min-h-0">
        <DataTable
            data={attrs}
            {columns}
            clientSide
            searchable
            getRowId={a => a.name}
            {remove}
            {editable}
            onFetch={fetchAttrs}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={attrs.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                            <AttrAddNew onSave={onAddNew} {attrs} />
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>
