<script lang="ts">
    import { untrack } from 'svelte';
    import IconEnvelope from '$icons/IconEnvelope.svelte';
    import Button from '$lib/button/Button.svelte';
    import Modal from '$lib/Modal.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import EditorInteractive from '$lib/text_edit/EditorInteractive.svelte';
    import type { GroupResponse } from '$api/types/groups';
    import type { RoleResponse } from '$api/types/roles';
    import Input from '$lib/form/Input.svelte';
    import type {
        ChannelInfo,
        EmailJobRequest,
        EmailTargets,
        MessageChannel,
    } from '$api/types/email';
    import InputArea from '$lib/form/InputArea.svelte';
    import InputDateTimeCombo from '$lib/form/InputDateTimeCombo.svelte';
    import { fmtDateInput, fmtTimeInput, unixTsFromLocalDateTime } from '$utils/form';
    import InputCheckbox from '$lib/form/InputCheckbox.svelte';
    import { fetchGet, fetchPost } from '$api/fetch';
    import IconCheck from '$icons/IconCheck.svelte';
    import { useSession } from '$state/session.svelte';
    import SelectList from '$lib5/select_list/SelectList.svelte';
    import type { SelectItem } from '$lib5/select_list/props.ts';
    import { Badge } from '$lib/components/ui/badge';
    import X from '@lucide/svelte/icons/x';

    export interface EmailRecipients {
        roles?: string[];
        groups?: string[];
        users?: { id: string; label: string }[];
    }

    let {
        groups: groupsProp,
        roles: rolesProp,
        // controlled open state, so tables can open the dialog from a row / selection action
        open = $bindable(false),
        // pre-selected recipients, applied every time the dialog opens
        initial,
        // hide the toolbar envelope button when the dialog is opened from somewhere else
        trigger = true,
    }: {
        groups?: GroupResponse[];
        roles?: RoleResponse[];
        open?: boolean;
        initial?: EmailRecipients;
        trigger?: boolean;
    } = $props();

    let ta = useI18nAdmin();
    let session = useSession('admin');

    // A delegated group admin may only email members of groups it manages. The backend enforces
    // the same, so the role / user / exclude pickers are hidden for them.
    let isGroupAdmin = $derived(!session.isAdmin());

    // The lists are optional: pages that do not hold both of them let the dialog fetch what is
    // missing when it opens.
    let fetchedRoles: RoleResponse[] = $state([]);
    let fetchedGroups: GroupResponse[] = $state([]);
    let roles = $derived(rolesProp ?? fetchedRoles);
    let groups = $derived(groupsProp ?? fetchedGroups);
    let availableGroups = $derived(
        isGroupAdmin ? groups.filter(g => session.managesGroup(g.name)) : groups,
    );

    let closeModal: undefined | (() => void) = $state();

    let editorMode: 'Text' | 'Markdown' | 'HTML' = $state('Markdown');

    // channels the backend has a driver for; the selector only shows up if there is a choice
    let channels: MessageChannel[] = $state(['email']);
    let channel: MessageChannel = $state('email');
    // channels that send styled mail (the default mailer and own-server smtp channels) get the full editor
    let htmlChannels: string[] = $state(['email']);
    let isEmail = $derived(htmlChannels.includes(channel));
    // names from the backend, channels can be added by an admin
    let channelLabels: Record<MessageChannel, string> = $state({
        email: 'E-Mail',
        sms: 'SMS',
        whatsapp: 'WhatsApp',
    });

    let roleItems: SelectItem[] = $state([]);
    let groupItems: SelectItem[] = $state([]);
    let notRoleItems: SelectItem[] = $state([]);
    let notGroupItems: SelectItem[] = $state([]);
    let users: { id: string; label: string }[] = $state([]);

    let scheduled = $state(false);
    let schedDate = $state(fmtDateInput());
    let schedTime = $state(fmtTimeInput());

    let subject = $state('');
    let body = $state('');
    let bodyRaw = $state('');

    let success = $state(false);
    let error = $state('');
    let sending = $state(false);

    $effect(() => {
        if (open) {
            untrack(onOpen);
        }
    });

    async function onOpen() {
        error = '';
        success = false;
        users = [...(initial?.users ?? [])];
        rebuildItems();

        let ch = await fetchGet<ChannelInfo[]>('/auth/v1/email/channels');
        if (ch.body && ch.body.length > 0) {
            channels = ch.body.map(c => c.channel);
            for (const c of ch.body) channelLabels[c.channel] = c.channel === 'email' ? 'E-Mail' : c.label;
            htmlChannels = ch.body.filter(c => c.html).map(c => c.channel);
            if (!channels.includes(channel)) channel = channels[0];
        }

        if (!rolesProp && !isGroupAdmin) {
            let res = await fetchGet<RoleResponse[]>('/auth/v1/roles');
            if (res.body) fetchedRoles = res.body;
        }
        if (!groupsProp) {
            let res = await fetchGet<GroupResponse[]>('/auth/v1/groups');
            if (res.body) fetchedGroups = res.body;
        }
        rebuildItems();
    }

    // keeps what was ticked so far when the lists arrive after the dialog is already open
    function rebuildItems() {
        const keep = (items: SelectItem[], preset: string[] | undefined) =>
            new Set([...items.filter(i => i.selected).map(i => i.name), ...(preset ?? [])]);
        const build = (names: string[], chosen: Set<string>): SelectItem[] =>
            names.map(name => ({ name, selected: chosen.has(name) }));

        const rn = roles.map(r => r.name);
        const gn = availableGroups.map(g => g.name);
        const firstRoles = roleItems.length === 0 ? initial?.roles : undefined;
        const firstGroups = groupItems.length === 0 ? initial?.groups : undefined;
        roleItems = build(rn, keep(roleItems, firstRoles));
        groupItems = build(gn, keep(groupItems, firstGroups));
        notRoleItems = build(rn, keep(notRoleItems, undefined));
        notGroupItems = build(gn, keep(notGroupItems, undefined));
    }

    const names = (items: SelectItem[]) => items.filter(i => i.selected).map(i => i.name);

    let targets: EmailTargets = $derived({
        roles: isGroupAdmin ? [] : names(roleItems),
        groups: names(groupItems),
        users: isGroupAdmin ? [] : users.map(u => u.id),
        not_roles: isGroupAdmin ? [] : names(notRoleItems),
        not_groups: isGroupAdmin ? [] : names(notGroupItems),
    });
    let hasIncludes = $derived(
        targets.roles.length > 0 || targets.groups.length > 0 || targets.users.length > 0,
    );
    let hasExcludes = $derived(targets.not_roles.length > 0 || targets.not_groups.length > 0);
    // a group admin must always pick at least one of its groups, never "everyone"
    let recipientsInvalid = $derived(isGroupAdmin && targets.groups.length === 0);

    let summary = $derived.by(() => {
        const parts: string[] = [];
        if (targets.roles.length) parts.push(`role ${targets.roles.join(', ')}`);
        if (targets.groups.length) parts.push(`group ${targets.groups.join(', ')}`);
        if (targets.users.length) parts.push(`${targets.users.length} selected user(s)`);
        let s = parts.length ? `Users in ${parts.join(' or ')}` : 'All users';
        const ex: string[] = [];
        if (targets.not_roles.length) ex.push(`role ${targets.not_roles.join(', ')}`);
        if (targets.not_groups.length) ex.push(`group ${targets.not_groups.join(', ')}`);
        if (ex.length) s += `, except those in ${ex.join(' or ')}`;
        return s;
    });

    async function submit() {
        error = '';
        sending = true;

        let payload: EmailJobRequest = {
            channel,
            filter_type: 'none',
            content_type: !isEmail || editorMode === 'Text' ? 'text' : 'markdown',
            // SMS / WhatsApp have no subject
            subject: isEmail ? subject : '',
            body: isEmail && editorMode === 'HTML' ? body : bodyRaw,
        };
        if (hasIncludes || hasExcludes) {
            payload.targets = targets;
        }
        if (scheduled) {
            payload.scheduled = unixTsFromLocalDateTime(schedDate, schedTime);
        }

        let res = await fetchPost('/auth/v1/email', payload);
        sending = false;
        if (res.status === 200) {
            success = true;
            setTimeout(() => {
                closeModal?.();
                reset();
            }, 2000);
        } else {
            error = res.error?.message || 'Error';
        }
    }

    function reset() {
        scheduled = false;
        subject = '';
        bodyRaw = '';
        body = '';
        success = false;
        error = '';
        roleItems = [];
        groupItems = [];
        notRoleItems = [];
        notGroupItems = [];
        users = [];
    }
</script>

{#if trigger}
    <Button ariaLabel={ta.email.sendMail} onclick={() => (open = true)} invisible>
        <IconEnvelope width="1.3rem" />
    </Button>
{/if}

<Modal bind:showModal={open} bind:closeModal class="sm:max-w-3xl">
    <h3 class="text-lg font-semibold">{isEmail ? ta.email.sendMail : `Send ${channelLabels[channel]}`}</h3>

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

        <div class="grid gap-2 rounded-lg border p-3">
            <span class="text-sm font-medium">{ta.email.userFilter}</span>
            <p class="text-muted-foreground text-xs">
                {isGroupAdmin
                    ? 'Choose the groups you manage.'
                    : 'Users matching any selected role, group or user receive the email. Select nothing to email all users.'}
            </p>

            {#if !isGroupAdmin}
                <SelectList bind:items={roleItems} maxWidth="100%">Roles</SelectList>
            {/if}
            <SelectList bind:items={groupItems} maxWidth="100%">Groups</SelectList>

            {#if !isGroupAdmin}
                <div class="flex w-full items-start gap-2">
                    <span class="w-20 shrink-0 pt-1 text-sm font-medium">Users</span>
                    <div class="flex min-h-7 flex-1 flex-wrap items-center gap-1.5">
                        {#if users.length === 0}
                            <span class="text-muted-foreground text-sm">—</span>
                        {/if}
                        {#each users as u (u.id)}
                            <Badge variant="secondary" class="gap-1">
                                {u.label}
                                <button
                                    type="button"
                                    aria-label="Remove {u.label}"
                                    class="hover:text-destructive"
                                    onclick={() => (users = users.filter(x => x.id !== u.id))}
                                >
                                    <X class="size-3" />
                                </button>
                            </Badge>
                        {/each}
                    </div>
                </div>

                <details class="text-sm">
                    <summary class="text-muted-foreground cursor-pointer">Exclude</summary>
                    <div class="mt-2 grid gap-2">
                        <SelectList bind:items={notRoleItems} maxWidth="100%">Roles</SelectList>
                        <SelectList bind:items={notGroupItems} maxWidth="100%">Groups</SelectList>
                    </div>
                </details>
            {/if}
        </div>

        {#if channels.length > 1}
            <div class="flex flex-wrap items-center gap-2">
                <span class="text-sm font-medium">Channel</span>
                {#each channels as c (c)}
                    <Button
                        level={channel === c ? 1 : 3}
                        onclick={() => (channel = c)}
                        ariaLabel={channelLabels[c]}
                    >
                        {channelLabels[c]}
                    </Button>
                {/each}
            </div>
        {/if}

        {#if isEmail}
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
        {:else}
            <InputArea label="Message" placeholder="Message" bind:value={bodyRaw} rows={6} />
            <p class="text-muted-foreground text-xs">
                Plain text only. Users without a phone number are skipped.
            </p>
        {/if}

        <div class="grid gap-2">
            <p class="text-muted-foreground text-sm">
                {isEmail ? ta.email.sendAllUsersFiltered : `This ${channelLabels[channel]} message will be sent to:`}
                <b class="text-foreground">{summary}</b>
            </p>

            <div class="flex items-center gap-3">
                <Button
                    onclick={submit}
                    isLoading={sending}
                    isDisabled={!(body || bodyRaw) || (isEmail && !subject) || recipientsInvalid}
                >
                    Send
                </Button>

                {#if success}
                    <IconCheck width="1.4rem" />
                {:else if error}
                    <span class="text-destructive text-sm">{error}</span>
                {/if}
            </div>
        </div>
    </div>
</Modal>
