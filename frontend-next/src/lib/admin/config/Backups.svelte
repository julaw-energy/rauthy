<script lang="ts">
    import Msg from '$lib5/Msg.svelte'
    ;import DataTable from '$lib/components/data-table/data-table.svelte'
    ;import { componentCol, dateCol } from '$lib/components/data-table/columns'
    ;import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import BackupName from '$lib5/admin/config/BackupName.svelte';
    import type { BackupListing, BackupListings } from '$api/types/backup';
    import { fetchGet, fetchPost } from '$api/fetch';
    import { onMount } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { formatDateFromTs, prettyFileSize } from '$utils/helpers';
    import Button from '$lib/button/Button.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import IconArrowPathSquare from '$icons/IconArrowPathSquare.svelte';
    import A from '$lib/A.svelte';

    let ta = useI18nAdmin();

    let local: BackupListing[] = $state([]);
    let s3: BackupListing[] = $state([]);

    let isDisabled = $state(false);
    let success = $state(false);

    onMount(() => {
        fetchListings();
    });

    async function postCreateBackup() {
        let res = await fetchPost('/auth/v1/backup');
        if (res.error) {
            console.error(res.error);
        } else {
            success = true;

            // S3 pushes are done async in the background
            setTimeout(() => {
                fetchListings();
            }, 1000);

            setTimeout(() => {
                success = false;
            }, 5000);
        }
    }

    async function fetchListings() {
        let res = await fetchGet<BackupListings>('/auth/v1/backup');
        if (res.body) {
            res.body.local.sort((a, b) => b.last_modified - a.last_modified);
            res.body.s3.sort((a, b) => b.last_modified - a.last_modified);

            local = res.body.local;
            s3 = res.body.s3;
        } else if (res.error) {
            if (res.error.error === 'NotFound') {
                isDisabled = true;
            } else {
                console.error(res);
            }
        }
    }

    type BackupRow = BackupListing & { hrefPrefix: string };
    const backupColumns: FeaturedColumnDef<BackupRow>[] = [
        componentCol<BackupRow, Record<string, never>>('name', ta.backup.name, BackupName, undefined, { size: 360 }),
        dateCol<BackupRow>('last_modified', ta.backup.lastModified),
        {
            id: 'size',
            header: ta.backup.size,
            size: 130,
            accessorFn: (r: BackupRow) => (r.size ? prettyFileSize(r.size) : '-'),
        },
    ];
</script>

{#snippet table(rows: BackupListing[], hrefPrefix: string)}
    <DataTable
        data={rows.map(r => ({ ...r, hrefPrefix }))}
        columns={backupColumns}
        clientSide
        initialPageSize={10}
        initialSorting={[{ id: 'last_modified', desc: true }]}
        getRowId={r => r.name}
        emptyText={ta.common.noEntries}
    />
{/snippet}

<div class="grid max-w-4xl gap-4">
    {#if isDisabled}
        <p class="text-muted-foreground text-sm">{ta.backup.disabledDesc}</p>
    {:else}
        <div class="flex items-center gap-2">
            <Button onclick={postCreateBackup}>
                {ta.backup.createBackup}
            </Button>
            <Button invisible onclick={fetchListings} ariaLabel="Refresh">
                <IconArrowPathSquare width="1.25rem" />
            </Button>
            <Msg {success} />
        </div>

        {#if local.length === 0 && s3.length === 0}
            <p class="text-muted-foreground text-sm">{ta.common.noEntries}</p>
        {:else}
            <h2 class="text-xl font-semibold">{ta.backup.local}</h2>
            {@render table(local, '/auth/v1/backup/local')}

            <h2 class="text-xl font-semibold">S3</h2>
            {@render table(s3, '/auth/v1/backup/s3')}
        {/if}
    {/if}
</div>
