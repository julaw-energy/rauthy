<script lang="ts">
    import { onMount } from 'svelte';
    import { fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type {
        NewUsersSettingsResponse,
        ReminderSettings,
        ReminderSettingsResponse,
        SettingsTestResponse,
    } from '$api/types/settings';
    import FormSection from '$lib5/FormSection.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputArea from '$lib5/form/InputArea.svelte';
    import Msg from '$lib5/Msg.svelte';
    import Switch from '$lib5/Switch.svelte';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';

    // ---- roles and groups every new user gets ----
    let availableRoles: string[] = $state([]);
    let availableGroups: string[] = $state([]);
    let roles: string[] = $state([]);
    let groups: string[] = $state([]);
    let busyDefaults = $state(false);
    let errDefaults = $state('');
    let okDefaults = $state(false);

    // ---- reminder for users who have not logged in ----
    let reminder: ReminderSettings | undefined = $state();
    let afterDays = $state('30');
    let maxReminders = $state('3');
    let dueNow = $state(0);
    let busyReminder = $state(false);
    let errReminder = $state('');
    let okReminder = $state('');

    onMount(async () => {
        let d = await fetchGet<NewUsersSettingsResponse>('/auth/v1/settings/new_users');
        if (d.body) applyDefaults(d.body);
        else errDefaults = d.error?.message || 'The settings could not be loaded';

        let r = await fetchGet<ReminderSettingsResponse>('/auth/v1/settings/reminders');
        if (r.body) applyReminder(r.body);
        else errReminder = r.error?.message || 'The settings could not be loaded';
    });

    function applyDefaults(r: NewUsersSettingsResponse) {
        availableRoles = r.available_roles;
        availableGroups = r.available_groups;
        // keep only what still exists
        roles = r.values.roles.filter(x => r.available_roles.includes(x));
        groups = r.values.groups.filter(x => r.available_groups.includes(x));
    }

    function applyReminder(r: ReminderSettingsResponse) {
        reminder = r.values;
        afterDays = String(r.values.after_days);
        maxReminders = String(r.values.max_reminders);
        dueNow = r.due_now;
    }

    function toggle(list: string[], name: string, on: boolean): string[] {
        return on ? [...new Set([...list, name])] : list.filter(x => x !== name);
    }

    async function saveDefaults() {
        busyDefaults = true;
        errDefaults = '';
        okDefaults = false;
        let res = await fetchPut<NewUsersSettingsResponse>('/auth/v1/settings/new_users', {
            roles,
            groups,
        });
        if (res.body) {
            applyDefaults(res.body);
            okDefaults = true;
            setTimeout(() => (okDefaults = false), 2500);
        } else {
            errDefaults = res.error?.message || 'Saving failed';
        }
        busyDefaults = false;
    }

    function reminderPayload(): ReminderSettings | undefined {
        if (!reminder) return undefined;
        return {
            ...reminder,
            after_days: Number(afterDays),
            max_reminders: Number(maxReminders),
        };
    }

    async function saveReminder() {
        let p = reminderPayload();
        if (!p) return;
        if (!Number.isInteger(p.after_days) || !Number.isInteger(p.max_reminders)) {
            errReminder = 'The days and the number of reminders must be whole numbers';
            return;
        }
        busyReminder = true;
        errReminder = '';
        okReminder = '';
        let res = await fetchPut<ReminderSettingsResponse>('/auth/v1/settings/reminders', p);
        if (res.body) {
            applyReminder(res.body);
            okReminder = 'Saved';
        } else {
            errReminder = res.error?.message || 'Saving failed';
        }
        busyReminder = false;
    }

    async function testReminder() {
        busyReminder = true;
        errReminder = '';
        okReminder = '';
        // the test uses what is saved, so save first
        let p = reminderPayload();
        if (p) {
            let saved = await fetchPut<ReminderSettingsResponse>('/auth/v1/settings/reminders', p);
            if (saved.error) {
                errReminder = saved.error.message;
                busyReminder = false;
                return;
            }
            if (saved.body) applyReminder(saved.body);
        }
        let res = await fetchPost<SettingsTestResponse>('/auth/v1/settings/reminders/test');
        if (res.body) okReminder = res.body.message;
        else errReminder = res.error?.message || 'The test failed';
        busyReminder = false;
    }
</script>

<div class="grid max-w-3xl gap-4">
    <div>
        <h2 class="text-xl font-semibold tracking-tight">New users</h2>
        <p class="text-muted-foreground text-sm">
            What a user gets when they join, and what happens when they stay away.
        </p>
    </div>

    <FormSection
        title="Roles and groups for new users"
        desc="Every new user gets these. New users are the ones who register themselves and the ones created by their first login through an auth provider. Users that an admin creates are not changed. <code>rauthy_admin</code> cannot be chosen here."
    >
        <div class="grid gap-2">
            <span class="text-sm font-medium">Roles</span>
            {#if availableRoles.length === 0}
                <p class="text-muted-foreground text-sm">There are no roles to choose from.</p>
            {/if}
            <div class="grid gap-2 sm:grid-cols-3">
                {#each availableRoles as name (name)}
                    <Label class="inline-flex items-center gap-2 font-normal">
                        <Checkbox
                            checked={roles.includes(name)}
                            onCheckedChange={v => (roles = toggle(roles, name, v === true))}
                            aria-label={name}
                        />
                        <span>{name}</span>
                    </Label>
                {/each}
            </div>
        </div>

        <div class="grid gap-2">
            <span class="text-sm font-medium">Groups</span>
            {#if availableGroups.length === 0}
                <p class="text-muted-foreground text-sm">There are no groups to choose from.</p>
            {/if}
            <div class="grid gap-2 sm:grid-cols-3">
                {#each availableGroups as name (name)}
                    <Label class="inline-flex items-center gap-2 font-normal">
                        <Checkbox
                            checked={groups.includes(name)}
                            onCheckedChange={v => (groups = toggle(groups, name, v === true))}
                            aria-label={name}
                        />
                        <span>{name}</span>
                    </Label>
                {/each}
            </div>
        </div>

        <div class="flex flex-wrap items-center gap-2">
            <Button onclick={saveDefaults} isLoading={busyDefaults}>Save</Button>
            {#if okDefaults}<span class="text-sm">Saved</span>{/if}
        </div>
        <Msg err={errDefaults} />
    </FormSection>

    <FormSection
        title="Reminder for users who have not logged in"
        desc="A user who did not log in for a number of days gets an email, then another one after the same time, up to the number below, until they log in again. A user who never logged in counts from the day the account was created. At most 100 users are reminded per hour."
    >
        {#if reminder}
            <Switch bind:checked={reminder.enabled} ariaLabel="Send reminders">
                Send reminders
            </Switch>
            {#if reminder.enabled && dueNow > 0}
                <p class="bg-muted rounded-md px-3 py-2 text-sm">
                    <b>{dueNow}</b>
                    {dueNow === 1 ? 'user is' : 'users are'} due for a reminder right now. They get it
                    over the next hours, 100 per hour at most.
                </p>
            {:else if !reminder.enabled}
                <p class="text-muted-foreground text-sm">
                    Nothing is sent while this is off. {dueNow}
                    {dueNow === 1 ? 'user would be' : 'users would be'} due with the values below.
                </p>
            {/if}

            <div class="grid gap-3 sm:grid-cols-2">
                <Input
                    typ="number"
                    bind:value={afterDays}
                    label="Days without a login"
                    autocomplete="off"
                    min="1"
                    max="3650"
                />
                <Input
                    typ="number"
                    bind:value={maxReminders}
                    label="Reminders in a row"
                    autocomplete="off"
                    min="1"
                    max="10"
                />
            </div>

            <Input bind:value={reminder.subject} label="Subject" autocomplete="off" maxLength={200} />
            <InputArea bind:value={reminder.body} label="Text" rows={9} maxLength={5000} />
            <p class="text-muted-foreground text-xs">
                Placeholders: <code>{'{{name}}'}</code>, <code>{'{{days}}'}</code> (days since the last
                login), <code>{'{{link}}'}</code> (the account page) and <code>{'{{app_name}}'}</code>. The
                subject can use all but the link. The same text goes to every language.
            </p>

            <div class="flex flex-wrap items-center gap-2">
                <Button onclick={saveReminder} isLoading={busyReminder}>Save</Button>
                <Button level={2} onclick={testReminder} isDisabled={busyReminder}>
                    Save and send a test to me
                </Button>
            </div>
            <Msg err={errReminder} success={!!okReminder} />
            {#if okReminder}<p class="text-sm">{okReminder}</p>{/if}
        {/if}
    </FormSection>
</div>
