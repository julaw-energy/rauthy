<script lang="ts">
    import Button from '$lib/button/Button.svelte';
    import IconCircleStack from '$icons/IconCircleStack.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Modal from '$lib/Modal.svelte';
    import type { EmailJobResponse } from '$api/types/email';
    import { fetchGet, fetchPost } from '$api/fetch';
    import { formatDateFromTs } from '$utils/helpers';
    import IconClock from '$icons/IconClock.svelte';
    import Expandable from '$lib/Expandable.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import IconStop from '$icons/IconStop.svelte';
    import LabeledValue from '$lib/LabeledValue.svelte';
    import MarkdownRenderer from '$lib/text_edit/md/MarkdownRenderer.svelte';

    let ta = useI18nAdmin();

    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    let jobs: EmailJobResponse[] = $state([]);

    $effect(() => {
        if (showModal) {
            fetchJobs();
        }
    });

    async function fetchJobs() {
        let res = await fetchGet<EmailJobResponse[]>('/auth/v1/email');
        if (res.body) {
            jobs = res.body;
        } else {
            console.error(res.error);
        }
    }

    async function cancelJob(job: EmailJobResponse) {
        if (job.status !== 'Open') {
            return;
        }

        let res = await fetchPost(`/auth/v1/email/cancel/${job.id}`);
        if (res.error) {
            console.error(res.error);
        } else {
            await fetchJobs();
        }
    }
</script>

<Button onclick={() => (showModal = true)} invisible ariaLabel={ta.email.jobs}>
    <IconCircleStack width="1.3rem" />
</Button>

<Modal bind:showModal bind:closeModal class="sm:max-w-3xl">
    <h3 class="text-lg font-semibold">{ta.email.jobs}</h3>

    <div class="grid max-h-[65dvh] gap-2 overflow-y-auto">
        {#if jobs.length === 0}
            <p class="text-muted-foreground">-</p>
        {/if}

        {#each jobs as job (job.id)}
            <div class="rounded-xl border p-1">
                <Expandable>
                    {#snippet summary()}
                        <div class="grid gap-1">
                            <div class="flex flex-wrap items-center gap-x-4 gap-y-1">
                                <span>{formatDateFromTs(job.id)}</span>
                                <span class="flex items-center gap-1">
                                    {#if job.status === 'Finished'}
                                        <IconCheck width="1.1rem" />
                                    {:else if job.status === 'Open'}
                                        <IconClock width="1.1rem" />
                                    {:else if job.status === 'Canceled'}
                                        <IconStop width="1.1rem" />
                                    {/if}
                                    {job.status}
                                    {#if job.status !== 'Open'}
                                        <span class="text-muted-foreground">({formatDateFromTs(job.updated)})</span>
                                    {/if}
                                </span>
                                {#if job.status === 'Open'}
                                    <Button
                                        ariaLabel={ta.email.cancelJob}
                                        invisible
                                        level={-3}
                                        onclick={() => cancelJob(job)}
                                    >
                                        <IconStop width="1.1rem" />
                                    </Button>
                                {/if}
                            </div>
                            <b class="font-semibold break-words">
                                {#if job.subject.length > 79}
                                    {job.subject.slice(0, 79)}(...)
                                {:else}
                                    {job.subject}
                                {/if}
                            </b>
                        </div>
                    {/snippet}
                    {#snippet details()}
                        <div class="grid gap-3">
                            <div class="grid gap-3 sm:grid-cols-3">
                                <LabeledValue label={ta.email.scheduled}>
                                    {#if job.scheduled}
                                        {formatDateFromTs(job.scheduled)}
                                    {:else}
                                        {ta.email.immediate}
                                    {/if}
                                </LabeledValue>
                                <LabeledValue label="Update">
                                    {formatDateFromTs(job.updated)}
                                </LabeledValue>
                                <LabeledValue label="Filter">
                                    {job.filter_type}
                                    {job.filter_value}
                                </LabeledValue>
                            </div>
                            <LabeledValue label={ta.email.subject}>
                                {job.subject}
                            </LabeledValue>

                            <div class="bg-muted/40 max-h-72 overflow-auto rounded-xl border p-3 text-sm whitespace-pre-wrap">
                                {#if job.content_type === 'text'}
                                    {job.body}
                                {:else if job.content_type === 'markdown'}
                                    <MarkdownRenderer markdown={job.body} />
                                {:else}
                                    {@html job.body}
                                {/if}
                            </div>
                        </div>
                    {/snippet}
                </Expandable>
            </div>
        {/each}
    </div>
</Modal>
