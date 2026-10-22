<script lang="ts">
    import { EVENT_LEVELS, EVENT_TYPES } from '$utils/constants';
    import type { EventLevel, EventResponse, EventsRequest, EventType } from '$api/types/events.ts';
    import { fetchPost } from '$api/fetch';
    import Options from '$lib5/Options.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import InputDateTimeCombo from '$lib5/form/InputDateTimeCombo.svelte';
    import { fmtDateInput, fmtTimeInput, unixTsFromLocalDateTime } from '$utils/form';
    import ContentAdmin from '$lib5/ContentAdmin.svelte';
    import { useTrigger } from '$state/callback.svelte';
    import DataTable from '$lib/components/data-table/data-table.svelte';
    import { dateCol, textCol } from '$lib/components/data-table/columns';
    import type { FeaturedColumnDef } from '$lib/components/data-table/data-table-features';
    import { eventColor } from '$utils/helpers';

    let ta = useI18nAdmin();

    let refOpts: undefined | HTMLButtonElement = $state();
    let tr = useTrigger();
    tr.set('navMain', () => refOpts?.focus());

    let events: EventResponse[] = $state([]);

    let fromDate = $state(fmtDateInput());
    let fromTime = $state(fmtTimeInput());
    let untilDate: undefined | string = $state('');
    let untilTime = $state('--:--');
    let level: EventLevel = $state('info');
    let typ: EventType = $state(EVENT_TYPES[0] as EventType);
    let fetching = $state(false);

    const columns: FeaturedColumnDef<EventResponse>[] = [
        dateCol<EventResponse>('timestamp', 'Timestamp', { ms: true }),
        textCol<EventResponse>('level', 'Level', {
            size: 110,
            cell: ({ getValue }) => getValue() as string,
        }),
        textCol<EventResponse>('typ', 'Type', { size: 190 }),
        textCol<EventResponse>('text', 'Content', { size: 360 }),
        textCol<EventResponse>('ip', 'IP', { size: 150 }),
    ];

    $effect(() => {
        // re-fetch whenever any filter changes
        fromDate;
        fromTime;
        untilDate;
        untilTime;
        level;
        typ;
        fetchData();
    });

    async function fetchData() {
        let from = unixTsFromLocalDateTime(fromDate, fromTime);
        if (!from) {
            console.error('from ts invalid', from);
            return;
        }
        let until: undefined | number;
        if (untilDate && untilTime) {
            until = unixTsFromLocalDateTime(untilDate, untilTime);
        }

        let payload: EventsRequest = {
            from,
            until,
            level,
            typ: typ !== EVENT_TYPES[0] ? typ : undefined,
        };
        fetching = true;
        let res = await fetchPost<EventResponse[]>('/auth/v1/events', payload);
        if (res.body) {
            events = res.body;
        } else {
            console.error(res.error);
        }
        fetching = false;
    }
</script>

<ContentAdmin>
    <div id="archive" class="grid min-h-0 gap-4">
        <div class="bg-card grid gap-4 rounded-xl border p-4 md:grid-cols-2 xl:grid-cols-4">
            <InputDateTimeCombo label={ta.common.from} bind:value={fromDate} bind:timeValue={fromTime} withTime />
            <InputDateTimeCombo
                label={ta.common.until}
                bind:value={untilDate}
                bind:timeValue={untilTime}
                withTime
                withDelete
            />
            <div class="grid gap-1.5">
                <span class="text-sm font-medium">{ta.events.eventLevel}</span>
                <Options bind:ref={refOpts} ariaLabel={ta.events.eventLevel} options={EVENT_LEVELS} bind:value={level} />
            </div>
            <div class="grid gap-1.5">
                <span class="text-sm font-medium">{ta.events.eventType}</span>
                <Options ariaLabel={ta.events.eventType} options={EVENT_TYPES} bind:value={typ} />
            </div>
        </div>

        <DataTable
            data={events}
            {columns}
            clientSide
            searchable
            getRowId={e => e.id}
            {fetching}
            onFetch={fetchData}
            emptyText="No events found"
            initialSorting={[{ id: 'timestamp', desc: true }]}
        />
    </div>
</ContentAdmin>
