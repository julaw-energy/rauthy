<script lang="ts">
    import { formatUtcTsFromDateInput } from '$utils/helpers.js';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { dateCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import type { RowDeletion } from '$lib/components/data-table/row-deletion';
    import Msg from '$lib5/Msg.svelte';
    import { onMount } from 'svelte';
    import AddFooter from '$lib5/form/AddFooter.svelte';
    import { toast } from 'svelte-sonner';
    import Input from '$lib5/form/Input.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import InputDateTimeCombo from '$lib5/form/InputDateTimeCombo.svelte';
    import { fmtDateInput, fmtTimeInput } from '$utils/form';
    import { fetchDelete, fetchGet, fetchPost } from '$api/fetch';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import type {
        BlacklistedIp,
        BlacklistResponse,
        IpBlacklistRequest,
    } from '$api/types/blacklist.ts';
    import ButtonAddModal from '$lib5/button/ButtonAddModal.svelte';
    import Form from '$lib5/form/Form.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import { useSession } from '$state/session.svelte';

    let t = useI18n();
    let ta = useI18nAdmin();

    // group admins get a read-only blacklist view for debugging; adding / removing entries
    // is full-admin only
    let session = useSession('admin');
    let isAdmin = $derived(session.isAdmin());

    let refOpts: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refOpts?.focus());

    let refIp: undefined | HTMLInputElement = $state();
    let closeModal: undefined | (() => void) = $state();

    let err = $state('');
    let errSave = $state('');
    let blacklist: BlacklistedIp[] = $state([]);

    let ip = $state('');
    // bumped after a save to remount the form, so the emptied required field is not flagged invalid
    let formNonce = $state(0);
    let another = $state(false);
    // The default expiry is a day from now: the old default was "now", so a new entry had already
    // expired when it was saved and vanished from the list right away.
    const inOneDay = () => new Date(Date.now() + 24 * 60 * 60 * 1000);
    let expDate = $state(fmtDateInput(inOneDay()));
    let expTime = $state(fmtTimeInput(inOneDay()));

    onMount(() => {
        fetchBlacklist();
    });

    $effect(() => {
        refIp?.focus();
    });

    async function fetchBlacklist() {
        let res = await fetchGet<BlacklistResponse>('/auth/v1/blacklist');
        if (res.body) {
            blacklist = res.body.ips;
        } else {
            err = res.error?.message || 'Error';
        }
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        errSave = '';

        let exp = formatUtcTsFromDateInput(expDate, expTime);
        if (!exp) {
            errSave = 'Invalid Date Input: User Expires';
            return;
        }
        if (exp * 1000 <= Date.now()) {
            errSave = 'The expiry must be in the future';
            return;
        }

        let payload: IpBlacklistRequest = {
            ip,
            exp,
        };

        // This is the only place we wrap the fetch in try, because we don't do validation
        // on the client side. A regex matching IPv4 + IPv6 is insanely huge.
        try {
            let res = await fetchPost(form.action, payload);
            if (res.error) {
                errSave = res.error.message;
            } else {
                ip = '';
                formNonce++;
                // the form stays mounted: give the next entry a fresh default expiry
                expDate = fmtDateInput(inOneDay());
                expTime = fmtTimeInput(inOneDay());
                if (another) {
                    toast.success('Saved');
                } else {
                    closeModal?.();
                }
                await fetchBlacklist();
            }
        } catch (err) {
            errSave = 'Invalid IP Address';
        }
    }

    const columns: FeaturedColumnDef<BlacklistedIp>[] = [
        textCol<BlacklistedIp>('ip', 'IP', { size: 260 }),
        dateCol<BlacklistedIp>('exp', 'Expires'),
    ];

    const remove: RowDeletion<BlacklistedIp> = {
        label: e => e.ip,
        verb: 'Remove',
        description: 'The IP will be removed from the blacklist.',
        run: async e => {
            let res = await fetchDelete(`/auth/v1/blacklist/${e.ip}`);
            if (res.error) throw new Error(res.error.message);
        },
        onDeleted: fetchBlacklist,
    };
</script>

<ContentAdmin>
    {#if err}
        <p class="text-destructive text-sm">{err}</p>
    {/if}

    <div id="blacklist" class="min-h-0">
        <DataTable
            data={blacklist}
            {columns}
            clientSide
            searchable
            searchPlaceholder="IP"
            getRowId={e => e.ip}
            remove={isAdmin ? remove : undefined}
            onFetch={fetchBlacklist}
            emptyText={ta.common.noEntries}
            initialSorting={[{ id: 'exp', desc: false }]}
        >
            {#snippet toolbar()}
                {#if isAdmin}
                    <div class="ml-auto">
                        <ButtonAddModal level={1} bind:closeModal>
                            <div class="grid gap-4">
                                {#key formNonce}
                                <Form action="/auth/v1/blacklist" {onSubmit}>
                                    <Input
                                        bind:ref={refIp}
                                        bind:value={ip}
                                        autocomplete="off"
                                        label="IP"
                                        placeholder="IPv4"
                                        required
                                        maxLength={40}
                                    />
                                    <InputDateTimeCombo
                                        label="Expiry"
                                        bind:value={expDate}
                                        bind:timeValue={expTime}
                                        min={fmtDateInput()}
                                        withTime
                                    />

                                    <AddFooter bind:another err={errSave} />
                                </Form>
                                {/key}
                            </div>
                        </ButtonAddModal>
                    </div>
                {/if}
            {/snippet}
        </DataTable>
    </div>
</ContentAdmin>
