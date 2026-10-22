<script lang="ts">
    import Event from '$lib5/admin/events/Event.svelte';
    import EventsLegend from './EventsLegend.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { EVENT_LEVELS } from '$utils/constants';
    import { isBrowser } from '$utils/helpers';
    import type { EventLevel, EventResponse } from '$api/types/events.ts';
    import Options from '$lib5/Options.svelte';
    import { ScrollArea } from '$lib/components/ui/scroll-area';
    import { onDestroy } from 'svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchPost } from '$api/fetch';
    import { useTrigger } from '$state/callback.svelte';
    import { useSession } from '$state/session.svelte';

    const latest = 50;

    let ta = useI18nAdmin();
    let session = useSession('admin');

    // The "Test" event verifies the notification pipeline, which is a full-admin concern
    // (fixing it needs config changes a group admin has no access to), so hide it for them.
    // The principal acts as a group admin when it reached the admin UI without the
    // full `rauthy_admin` role.
    let isGroupAdmin = $derived(!session.isAdmin());


    let es: undefined | EventSource = $state();
    let events: EventResponse[] = $state([]);
    let eventsFiltered: EventResponse[] = $state([]);
    let level: EventLevel = $state(
        isBrowser()
            ? (localStorage.getItem('eventLevel')?.toLowerCase() as EventLevel) || 'info'
            : 'info',
    );
    let levelBefore = '';

    onDestroy(() => {
        es?.close();
    });

    $effect(() => {
        if (level !== levelBefore) {
            levelBefore = level;
            stream();
        }
    });

    $effect(() => {
        switch (level) {
            case 'info':
                eventsFiltered = events;
                break;
            case 'notice':
                eventsFiltered = events.filter(
                    evt =>
                        evt.typ === 'Test' ||
                        evt.level === 'notice' ||
                        evt.level === 'warning' ||
                        evt.level === 'critical',
                );
                break;
            case 'warning':
                eventsFiltered = events.filter(
                    evt =>
                        evt.typ === 'Test' || evt.level === 'warning' || evt.level === 'critical',
                );
                break;
            case 'critical':
                eventsFiltered = events.filter(
                    evt => evt.typ === 'Test' || evt.level === 'critical',
                );
                break;
        }
    });

    async function sendTestEvent() {
        await fetchPost('/auth/v1/events/test');
    }

    function stream() {
        localStorage.setItem('eventLevel', level);

        if (es && es.readyState !== 2) {
            es.close();
        }

        es = new EventSource(
            `/auth/v1/events/stream?latest=${latest}&level=${level.toLowerCase()}`,
        );

        es.onopen = () => {
            events = [];
        };

        es.onerror = () => {
            // make sure to re-open after any error
            setTimeout(() => {
                stream();
            }, 3000);
        };

        es.onmessage = ev => {
            if (ev.data) {
                let event: EventResponse = JSON.parse(ev.data);
                // keep max 500 events in the UI to not consume endless amounts of memory
                events = [event, ...events.slice(-499)];
            }
        };
    }
</script>

<div id="events" class="bg-card flex h-full min-h-0 flex-col gap-2 rounded-xl border p-3">
    <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2">
            <b>Events</b>
            <div class="w-28">
                <Options ariaLabel={ta.events.eventLevel} options={EVENT_LEVELS} bind:value={level} borderless />
            </div>
        </div>
        {#if !isGroupAdmin}
            <Button ariaLabel="Test Event" level={3} onclick={sendTestEvent}>Test</Button>
        {/if}
    </div>
    <ScrollArea class="min-h-0 flex-1">
        <div aria-live="polite" aria-relevant="additions" class="grid gap-1.5 pr-3">
            {#each eventsFiltered as event (event.id)}
                <Event {event} />
            {/each}
        </div>
    </ScrollArea>
    <EventsLegend />
</div>
