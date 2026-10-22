<script lang="ts">
    import IconEnvelope from '$icons/IconEnvelope.svelte';
    import Button from '$lib/button/Button.svelte';
    import Modal from '$lib/Modal.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Options from '$lib/Options.svelte';
    import EditorInteractive from '$lib/text_edit/EditorInteractive.svelte';
    import type { GroupResponse } from '$api/types/groups';
    import type { RoleResponse } from '$api/types/roles';
    import Input from '$lib/form/Input.svelte';
    import type { EmailJobRequest } from '$api/types/email';
    import InputDateTimeCombo from '$lib/form/InputDateTimeCombo.svelte';
    import { fmtDateInput, fmtTimeInput, unixTsFromLocalDateTime } from '$utils/form';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { fetchPost } from '$api/fetch';
    import IconCheck from '$icons/IconCheck.svelte';
    import { useSession } from '$state/session.svelte';

    let {
        groups,
        roles,
    }: {
        groups: GroupResponse[];
        roles: RoleResponse[];
    } = $props();

    let ta = useI18nAdmin();
    let session = useSession('admin');

    // A delegated group admin may only email users of a single group it manages, so
    // the filter is locked to "In Group" and the group dropdown is limited to managed groups.
    // The backend enforces the same. The principal acts as a group admin when it reached the
    // admin UI without the full `rauthy_admin` role.
    let isGroupAdmin = $derived(!session.isAdmin());
    let availableGroups = $derived(
        isGroupAdmin ? groups.filter(g => session.managesGroup(g.name)) : groups,
    );

    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    let editorMode: 'Text' | 'Markdown' | 'HTML' = $state('Markdown');

    let filter = $state(ta.email.filterType[0]);
    let group = $state('');
    let role = $state('');

    let scheduled = $state(false);
    let schedDate = $state(fmtDateInput());
    let schedTime = $state(fmtTimeInput());

    let subject = $state('');
    let body = $state('');
    let bodyRaw = $state('');

    let success = $state(false);
    let error = $state('');

    let isFilterNone = $derived(filter === ta.email.filterType[0]);
    let isFilterGroup = $derived(
        filter === ta.email.filterType[1] || filter === ta.email.filterType[2],
    );
    let isFilterRole = $derived(
        filter === ta.email.filterType[3] || filter === ta.email.filterType[4],
    );

    $effect(() => {
        // default to the first selectable group, and keep `group` valid if the list narrows
        // for a group admin
        if (availableGroups.length > 0 && !availableGroups.some(g => g.name === group)) {
            group = availableGroups[0].name;
        }
    });

    $effect(() => {
        if (roles.length > 0) {
            role = roles[0].name;
        }
    });

    async function submit() {
        let payload: EmailJobRequest = {
            // scheduled: number;
            filter_type: 'none',
            content_type: editorMode === 'Text' ? 'text' : 'markdown',
            subject,
            body: editorMode === 'HTML' ? body : bodyRaw,
        };

        if (scheduled) {
            payload.scheduled = unixTsFromLocalDateTime(schedDate, schedTime);
        }
        if (isGroupAdmin) {
            // group admins are locked to an "In Group" filter for a group they manage
            payload.filter_type = 'in_group';
            payload.filter_value = group;
        } else if (isFilterGroup) {
            payload.filter_value = group;
            if (filter === ta.email.filterType[1]) {
                payload.filter_type = 'in_group';
            } else {
                payload.filter_type = 'not_in_group';
            }
        }
        if (!isGroupAdmin && isFilterRole) {
            payload.filter_value = role;
            if (filter === ta.email.filterType[3]) {
                payload.filter_type = 'has_role';
            } else {
                payload.filter_type = 'has_not_role';
            }
        }

        let res = await fetchPost('/auth/v1/email', payload);
        if (res.status === 200) {
            success = true;
            setTimeout(() => {
                closeModal?.();
                reset();
            }, 3000);
        } else {
            error = res.error?.message || 'Error';
        }
    }

    function reset() {
        scheduled = false;
        filter = ta.email.filterType[0];
        subject = '';
        bodyRaw = '';
        body = '';
        success = false;
        error = '';
    }
</script>

<Button ariaLabel={ta.email.sendMail} onclick={() => (showModal = true)} invisible>
    <IconEnvelope width="1.3rem" />
</Button>

<Modal bind:showModal bind:closeModal class="sm:max-w-3xl">
    <h3 class="text-lg font-semibold">{ta.email.sendMail}</h3>

    <div class="grid gap-4">
        <div class="grid gap-2">
            <InputCheckbox ariaLabel={ta.email.scheduled} bind:checked={scheduled}>
                {ta.email.scheduled}
            </InputCheckbox>

            {#if scheduled}
                <InputDateTimeCombo
                    bind:value={schedDate}
                    bind:timeValue={schedTime}
                    min={fmtDateInput()}
                    withTime
                />
            {/if}
        </div>

        <div class="flex flex-wrap items-center gap-2">
            <span class="text-sm font-medium">{ta.email.userFilter}:</span>

            {#if isGroupAdmin}
                <!-- a group admin is locked to "In Group" for a group it manages -->
                <span class="text-sm">{ta.email.filterType[1]}:</span>
                <div class="w-48">
                    <Options
                        ariaLabel={ta.email.userFilter}
                        options={availableGroups.map(g => g.name)}
                        bind:value={group}
                        withSearch={availableGroups.length > 7}
                    />
                </div>
            {:else}
                <div class="w-44">
                    <Options
                        ariaLabel={ta.email.userFilter}
                        options={ta.email.filterType}
                        bind:value={filter}
                    />
                </div>

                {#if filter !== ta.email.filterType[0]}
                    <span>:</span>
                    <div class="w-48">
                        {#if isFilterGroup}
                            <Options
                                ariaLabel={ta.email.userFilter}
                                options={groups.map(g => g.name)}
                                bind:value={group}
                                withSearch={groups.length > 7}
                            />
                        {:else if isFilterRole}
                            <Options
                                ariaLabel={ta.email.userFilter}
                                options={roles.map(g => g.name)}
                                bind:value={role}
                                withSearch={roles.length > 7}
                            />
                        {/if}
                    </div>
                {/if}
            {/if}
        </div>

        <Input
            label={ta.email.subject}
            placeholder={ta.email.subject}
            bind:value={subject}
            maxLength={1024}
            required
        />
        <EditorInteractive
            bind:mode={editorMode}
            bind:contentRaw={bodyRaw}
            bind:sanitizedValue={body}
            height="min(60dvh, 40rem)"
        />

        <div class="grid gap-2">
            <p class="text-muted-foreground text-sm">
                {#if isGroupAdmin}
                    {ta.email.sendAllUsersFiltered}
                    <b class="text-foreground">{`${ta.email.filterType[1]} ${group}`}</b>
                {:else if isFilterNone}
                    {ta.email.sendAllUsers}
                {:else if isFilterGroup}
                    {ta.email.sendAllUsersFiltered}
                    <b class="text-foreground">{`${filter} ${group}`}</b>
                {:else if isFilterRole}
                    {ta.email.sendAllUsersFiltered}
                    <b class="text-foreground">{`${filter} ${role}`}</b>
                {/if}
            </p>

            <div class="flex items-center gap-3">
                <Button onclick={submit} isDisabled={!(body || bodyRaw) || !subject}>Send</Button>

                {#if success}
                    <IconCheck width="1.4rem" />
                {:else if error}
                    <span class="text-destructive text-sm">{error}</span>
                {/if}
            </div>
        </div>
    </div>
</Modal>
