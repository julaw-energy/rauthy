<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import { useTrigger } from '$state/callback.svelte';
    import type { GroupResponse } from '$api/types/groups.ts';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { fnTextCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import type { EditConfig } from '$lib/components/data-table/data-table.svelte';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import { PATTERN_GROUP } from '$utils/patterns';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import SendMail from '$lib5/SendMail.svelte';
    import { Mail } from '@lucide/svelte';
    import Button from '$lib/components/ui/button/button.svelte';
    import GroupAddNew from '$lib5/admin/groups/GroupAddNew.svelte';

    let ta = useI18nAdmin();

    let refAddNew: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refAddNew?.focus());

    // bumping this re-creates the add form, so "save & add another" starts with an empty one
    let addNonce = $state(0);

    // "Email" row / selection action: one shared dialog, pre-filled with the chosen groups
    let mailOpen = $state(false);
    let mailInitial: { groups: string[] } = $state({ groups: [] });
    function emailTo(names: string[]) {
        mailInitial = { groups: names };
        mailOpen = true;
    }
    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let groups: GroupResponse[] = $state([]);

    const metaText = (x: GroupResponse) => (x.meta === undefined || x.meta === null ? '' : JSON.stringify(x.meta));

    const columns: FeaturedColumnDef<GroupResponse>[] = [
        textCol<GroupResponse>('id', 'ID', { size: 260 }),
        textCol<GroupResponse>('name', ta.common.name, { size: 260 }),
        fnTextCol<GroupResponse>('meta', 'Metadata (JSON)', x => metaText(x), { size: 420 }),
    ];

    const editable: EditConfig<GroupResponse> = {
        columns: {
            meta: {
                read: metaText,
                placeholder: '{"key": "value"}',
                validate: v => {
                    try {
                        JSON.parse(v);
                        return null;
                    } catch {
                        return 'Not valid JSON';
                    }
                },
            },
            name: { required: true, maxLength: 32, pattern: PATTERN_GROUP } },
        canEdit: x => !(false),
        save: async (x, changes) => {
            if (changes.name !== undefined && groups.some(o => o.id !== x.id && o.name === changes.name)) {
                throw new Error(ta.common.nameExistsAlready);
            }
            let res = await fetchPut(`/auth/v1/groups/${x.id}`, { group: changes.name ?? x.name, meta: metaFrom(changes.meta, x.meta) });
            if (res.error) throw new Error(res.error.message);
        },
        onSaved: fetchGroups,
    };

    const metaFrom = (text: string | undefined, current: GroupResponse['meta']) =>
        text === undefined ? current : text.trim() === '' ? undefined : JSON.parse(text);

    const remove: RowDeletion<GroupResponse> = {
        label: x => x.name,
        canRemove: x => !(false),
        run: async x => {
            let res = await fetchDelete(`/auth/v1/groups/${x.id}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchGroups,
    };

    onMount(() => {
        fetchGroups();
    });

    async function fetchGroups() {
        let res = await fetchGet<GroupResponse[]>('/auth/v1/groups');
        if (res.body) {
            groups = res.body;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    function onSave() {
        fetchGroups();
    }

    async function onAddNew(id: string, another?: boolean) {
        if (another) {
            addNonce++;
            toast.success('Saved');
            await fetchGroups();
            return;
        }
        closeModal?.();
        await fetchGroups();
    }
</script>

<ContentAdmin>
    <SendMail bind:open={mailOpen} initial={mailInitial} trigger={false} />
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="groups" class="min-h-0">
        <DataTable
            data={groups}
            {columns}
            clientSide
            searchable
            getRowId={x => x.id}
            {editable}
            {remove}
            rowActionsWidth={40}
            onFetch={fetchGroups}
        >
            {#snippet rowActions(x)}
                <Button variant="ghost" size="icon-sm" aria-label="Send email" title="Send email" onclick={() => emailTo([x.name])}>
                    <Mail class="size-4" />
                </Button>
            {/snippet}
            {#snippet selectionActions(rows)}
                <Button size="sm" variant="outline" onclick={() => emailTo(rows.map(r => r.name))}>
                    <Mail class="size-4" />Email selected
                </Button>
            {/snippet}
            {#snippet toolbar()}
                <div class="ml-auto">
                    <ButtonAddModal bind:ref={refAddNew} level={groups.length === 0 ? 1 : 2} bind:closeModal alignRight>
                        {#key addNonce}
                            <GroupAddNew onSave={onAddNew} {groups} />
                        {/key}
                    </ButtonAddModal>
                </div>
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>
