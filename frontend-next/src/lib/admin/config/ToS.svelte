<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte.js';
    import { fetchGet, fetchPost } from '$api/fetch.js';
    import { onMount } from 'svelte';
    import Button from '$lib/button/Button.svelte';
    import Modal from '$lib/Modal.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import type { ToSRequest, ToSResponse, ToSUserAcceptResponse } from '$api/types/tos';
    import SearchBar from '$lib/search_bar/SearchBar.svelte';
    import { fetchSearchServer } from '$utils/search';
    import type { UserResponseSimple } from '$api/types/user';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { slide } from 'svelte/transition';
    import InputDateTimeCombo from '$lib/form/InputDateTimeCombo.svelte';
    import { fmtDateInput, fmtTimeInput } from '$utils/form';
    import { formatDateFromTs, formatUtcTsFromDateInput } from '$utils/helpers';
    import LabeledValue from '$lib/LabeledValue.svelte';
    import Options from '$lib/Options.svelte';
    import EditorInteractive from '$lib/text_edit/EditorInteractive.svelte';
    import MarkdownRenderer from '$lib/text_edit/md/MarkdownRenderer.svelte';

    let t = useI18n();
    let ta = useI18nAdmin();

    let refSearch: undefined | HTMLInputElement = $state();

    let showModalAddNew = $state(false);
    let closeModalAddNew: undefined | (() => void) = $state();
    let showModalStatus = $state(false);
    let closeModalStatus: undefined | (() => void) = $state();
    let isModalOpen = $derived(showModalAddNew || showModalStatus);

    let error = $state('');
    let noneExist = $state(false);
    let tos: ToSResponse[] = $state([]);
    let editorMode: 'HTML' | 'Text' | 'Markdown' = $state('Markdown');
    let newToSContentRaw = $state('');
    let optUntil = $state(false);
    let optUntilDate = $state(fmtDateInput());
    let optUntilTime = $state(fmtTimeInput());

    let selectOpts: string[] = $state([]);
    let selectedTsLabel = $state('');
    let selectedIdx = $state(-1);

    let searchValue = $state('');
    let searchOptions: UserResponseSimple[] = $state([]);
    let selectedId = $state('');
    let selectedEmail = $state('');
    let userStatus: undefined | ToSUserAcceptResponse[] = $state();

    onMount(() => {
        getTos();
    });

    $effect(() => {
        getUserStatus();
    });

    $effect(() => {
        searchUser();
    });

    $effect(() => {
        if (tos.length > 0) {
            let opts = tos.map(tos => formatDateFromTs(tos.ts));
            opts[0] = `${opts[0]} (Latest)`;

            selectOpts = opts;
            selectedTsLabel = opts[0];
        } else {
            selectOpts = [];
            selectedTsLabel = '';
        }
    });

    $effect(() => {
        if (selectedTsLabel) {
            selectedIdx = selectOpts.findIndex(o => o === selectedTsLabel);
        } else {
            selectedIdx = -1;
        }
    });

    $effect(() => {
        if (refSearch) {
            requestAnimationFrame(() => {
                refSearch?.focus();
            });
        }
    });

    $effect(() => {
        if (searchOptions.length === 1) {
            selectOpt(searchOptions[0]);
        }
    });

    async function searchUser() {
        if (searchValue.length < 3) {
            searchOptions = [];
            return;
        }

        let res = await fetchSearchServer<UserResponseSimple[]>({
            ty: 'user',
            idx: 'email',
            q: searchValue,
        });
        if (res.body) {
            searchOptions = res.body;
        } else {
            console.error(res.error);
        }
    }

    async function getUserStatus() {
        if (!selectedId) {
            userStatus = undefined;
            return;
        }

        let res = await fetchGet<ToSUserAcceptResponse[]>(`/auth/v1/tos/user/${selectedId}`);
        if (res.body) {
            userStatus = res.body;
        } else {
            console.error(res.error);
        }
    }

    async function getTos() {
        let res = await fetchGet<ToSResponse[]>('/auth/v1/tos');
        if (res.body) {
            // Always have the latest ToS at the top. The backend sends them ASC.
            res.body.reverse();
            tos = res.body;
        } else if (res.status === 204) {
            noneExist = true;
            selectedIdx = -1;
        }
    }

    async function saveToS() {
        error = '';

        let content = newToSContentRaw.trim();
        if (content.length === 0) {
            return;
        }

        let payload: ToSRequest = {
            is_html: editorMode !== 'Text',
            content,
        };
        if (optUntil) {
            payload.opt_until = formatUtcTsFromDateInput(optUntilDate, optUntilTime);
        }

        let res = await fetchPost('/auth/v1/tos', payload);
        if (res.error) {
            error = res.error.message;
        } else {
            closeModalAddNew?.();
            // we need a short timeout for the very first ToS
            setTimeout(() => {
                getTos();
            }, 500);
        }
    }

    function selectOpt(opt: UserResponseSimple) {
        selectedEmail = opt.email;
        selectedId = opt.id;
    }
</script>

<div class="grid max-w-4xl gap-4">
    <h2 class="text-xl font-semibold">{t.tos.tos}</h2>

    <div class="flex flex-wrap gap-2">
        <Button
            ariaLabel={ta.tos.addNewToS}
            level={isModalOpen ? 2 : 1}
            onclick={() => (showModalAddNew = true)}
        >
            {ta.tos.addNewToS}
        </Button>
        {#if selectedIdx > -1 && tos.length > 0}
            <Button
                ariaLabel={ta.tos.addNewToSFromCurrent}
                level={isModalOpen ? 2 : 1}
                onclick={() => {
                    newToSContentRaw = tos[selectedIdx].content;
                    showModalAddNew = true;
                }}
            >
                {ta.tos.addNewToSFromCurrent}
            </Button>
        {/if}
        {#if tos.length > 0}
            <Button
                ariaLabel={ta.tos.checkStatus}
                level={isModalOpen ? 3 : 2}
                onclick={() => (showModalStatus = true)}
            >
                {ta.tos.checkStatus}
            </Button>
        {/if}
    </div>

    {#if noneExist}
        <p class="text-muted-foreground text-sm">{ta.tos.noneExist}</p>
    {:else if tos.length > 0 && selectedIdx > -1}
        {@const active = tos[selectedIdx]}

        <div class="grid gap-3 sm:grid-cols-2">
            {#if selectedTsLabel && selectOpts.length > 1}
                <div class="sm:col-span-2">
                    <Options ariaLabel="Select ToS" options={selectOpts} bind:value={selectedTsLabel} />
                </div>
            {/if}

            <LabeledValue label={ta.tos.added} title={ta.tos.added}>
                {formatDateFromTs(active.ts)}
            </LabeledValue>

            {#if active.opt_until}
                <LabeledValue label={ta.tos.optUntil.label} title={ta.tos.optUntil.label}>
                    {formatDateFromTs(active.opt_until)}
                </LabeledValue>
            {/if}
        </div>

        <div class="bg-card rounded-xl border p-4 text-sm whitespace-pre-wrap">
            {#if active.is_html}
                <MarkdownRenderer markdown={active.content} />
            {:else}
                {active.content}
            {/if}
        </div>
    {/if}
</div>

<Modal bind:showModal={showModalAddNew} bind:closeModal={closeModalAddNew} class="sm:max-w-4xl">
    <div class="grid gap-4">
        <h3 class="text-lg font-semibold">{ta.tos.addNewToS}</h3>

        <div class="grid gap-3">
            <InputCheckbox ariaLabel={ta.tos.optUntil.enable} bind:checked={optUntil}>
                {ta.tos.optUntil.enable}
            </InputCheckbox>

            {#if optUntil}
                <div class="grid gap-2" transition:slide={{ duration: 150 }}>
                    <InputDateTimeCombo
                        min={fmtDateInput()}
                        timeMin={fmtTimeInput()}
                        withTime
                        bind:value={optUntilDate}
                        bind:timeValue={optUntilTime}
                    />

                    <p class="text-muted-foreground text-sm">
                        {ta.tos.optUntil.desc}
                    </p>
                </div>
            {/if}
        </div>

        <EditorInteractive
            bind:mode={editorMode}
            bind:contentRaw={newToSContentRaw}
            height="max(calc(100dvh - 24rem), 20rem)"
        />

        <p class="text-sm font-semibold">{ta.tos.immutable}</p>

        <div class="flex flex-wrap items-center gap-2">
            <Button
                ariaLabel={t.common.save}
                onclick={saveToS}
                isDisabled={newToSContentRaw.trim().length === 0}
            >
                {t.common.save}
            </Button>
            <Button level={-3} ariaLabel={t.common.cancel} onclick={() => closeModalAddNew?.()}>
                {t.common.cancel}
            </Button>
            <Msg err={error} />
        </div>
    </div>
</Modal>

{#if tos.length > 0}
    <Modal bind:showModal={showModalStatus} bind:closeModal={closeModalStatus}>
        <div class="grid gap-4">
            <h3 class="text-lg font-semibold">{ta.tos.checkStatus}</h3>

            <div class="grid gap-2">
                <SearchBar bind:ref={refSearch} placeholder="E-Mail" bind:value={searchValue} />
                <div class="grid max-h-48 gap-1 overflow-y-auto">
                    {#each searchOptions as opt (opt.id)}
                        <Button level={3} onclick={() => selectOpt(opt)} class="justify-start">
                            <span class={selectedId === opt.id ? 'text-primary font-semibold' : ''}>
                                {`${opt.given_name} ${opt.family_name || ''} <${opt.email}>`}
                            </span>
                        </Button>
                    {/each}
                </div>
            </div>

            {#if userStatus}
                <div class="grid gap-2">
                    <h4 class="font-semibold">{selectedEmail}</h4>

                    <div class="text-muted-foreground grid grid-cols-2 gap-2 border-b pb-1 text-sm font-medium">
                        <div>{t.tos.tos}</div>
                        <div>{ta.tos.accepted}</div>
                    </div>

                    {#each tos as tosEntry (tosEntry.ts)}
                        {@const stat = userStatus.find(s => s.tos_ts === tosEntry.ts)}

                        <div class="grid grid-cols-2 gap-2 text-sm">
                            <div>{formatDateFromTs(tosEntry.ts)}</div>
                            <div>
                                {#if stat}
                                    {formatDateFromTs(stat.accept_ts)}
                                    -
                                    {stat.location}
                                {:else}
                                    -
                                {/if}
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}

            <div>
                <Button level={2} onclick={() => closeModalStatus?.()}>
                    {t.common.close}
                </Button>
            </div>
        </div>
    </Modal>
{/if}
