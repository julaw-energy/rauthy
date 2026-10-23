<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import { isDefaultScope } from '$utils/helpers';
    import { useTrigger } from '$state/callback.svelte';
    import type { ScopeRequest, ScopeResponse } from '$api/types/scopes.ts';
    import type { UserAttrConfigResponse, UserAttrConfigValueResponse } from '$api/types/user_attrs.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import type { EditConfig } from '$lib/components/data-table/data-table.svelte';
    import { PATTERN_ROLE_SCOPE } from '$utils/patterns';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import ScopeAddNew from '$lib5/admin/scopes/ScopeAddNew.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);
    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let attrs: UserAttrConfigValueResponse[] = $state([]);
    let scopes: ScopeResponse[] = $state([]);

    const listText = (v?: string[]) => (v ?? []).join(', ');
    const parseList = (t: string) =>
        t
            .split(',')
            .map(x => x.trim())
            .filter(Boolean);

    const columns: FeaturedColumnDef<ScopeResponse>[] = [
        textCol<ScopeResponse>('id', 'ID', { size: 240 }),
        textCol<ScopeResponse>('name', ta.common.name, { size: 200 }),
        fnTextCol<ScopeResponse>('attr_include_access', 'Access Token', s => listText(s.attr_include_access), { size: 260 }),
        fnTextCol<ScopeResponse>('attr_include_id', 'ID Token', s => listText(s.attr_include_id), { size: 260 }),
        fnTextCol<ScopeResponse>('claims_at_root', ta.scopes.claimsAtRoot, s => (s.claims_at_root ? '✓' : '–'), { size: 150 }),
    ];

    // attribute names are chosen as a comma separated list; unknown names are rejected before saving
    const validateAttrs = (v: string) => {
        let known = attrs.map(a => a.name);
        let unknown = parseList(v).filter(n => !known.includes(n));
        return unknown.length ? `Unknown attribute: ${unknown.join(', ')}` : null;
    };

    const editable: EditConfig<ScopeResponse> = {
        columns: {
            name: { required: true, maxLength: 32, pattern: PATTERN_ROLE_SCOPE },
            attr_include_access: {
                placeholder: 'attr1, attr2',
                read: s => listText(s.attr_include_access),
                validate: validateAttrs,
            },
            attr_include_id: {
                placeholder: 'attr1, attr2',
                read: s => listText(s.attr_include_id),
                validate: validateAttrs,
            },
            claims_at_root: { kind: 'bool', read: s => String(!!s.claims_at_root) },
        },
        canEdit: x => !isDefaultScope(x.name),
        save: async (x, ch) => {
            let name = ch.name ?? x.name;
            if (isDefaultScope(name) || scopes.some(o => o.id !== x.id && o.name === name)) {
                throw new Error(ta.common.nameExistsAlready);
            }
            let payload: ScopeRequest = { scope: name };
            let access = parseList(ch.attr_include_access ?? listText(x.attr_include_access));
            let id = parseList(ch.attr_include_id ?? listText(x.attr_include_id));
            if (access.length > 0) payload.attr_include_access = access;
            if (id.length > 0) payload.attr_include_id = id;
            payload.claims_at_root = (ch.claims_at_root ?? String(!!x.claims_at_root)) === 'true';
            let res = await fetchPut(`/auth/v1/scopes/${x.id}`, payload);
            if (res.error) throw new Error(res.error.message);
        },
        onSaved: fetchScopes,
    };

    const remove: RowDeletion<ScopeResponse> = {
        label: x => x.name,
        canRemove: x => !isDefaultScope(x.name),
        run: async x => {
            let res = await fetchDelete(`/auth/v1/scopes/${x.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchScopes,
    };

    onMount(() => {
        fetchScopes();
        fetchAttrs();
    });

    async function fetchScopes() {
        let res = await fetchGet<ScopeResponse[]>('/auth/v1/scopes');
        if (res.body) {
            scopes = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function fetchAttrs() {
        let res = await fetchGet<UserAttrConfigResponse>('/auth/v1/users/attr');
        if (res.body) {
            attrs = res.body.values;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchScopes();
    }

    async function onAddNew(id: string, another?: boolean) {
        if (another) {
            addNonce++;
            toast.success('Saved');
            await fetchScopes();
            return;
        }
        closeModal?.();
        await fetchScopes();
    }
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="scopes" class="min-h-0">
        <DataTable
            data={scopes}
            {columns}
            clientSide
            searchable
            getRowId={s => s.id}
            {remove}
            {editable}
            onFetch={fetchScopes}
        >
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={scopes.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                            <ScopeAddNew onSave={onAddNew} {scopes} />
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>
