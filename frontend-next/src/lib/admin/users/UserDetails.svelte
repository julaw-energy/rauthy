<script lang="ts">
    import { useI18n } from '$state/i18n.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import Tabs from '$lib5/tabs/Tabs.svelte';
    import type { RoleResponse } from '$api/types/roles.ts';
    import type { GroupResponse } from '$api/types/groups.ts';
    import UserInfo from '$lib5/admin/users/UserInfo.svelte';
    import { fetchGet } from '$api/fetch';
    import type { UserResponse } from '$api/types/user.ts';
    import UserAttr from '$lib5/admin/users/UserAttr.svelte';
    import UserAddToGroups from './UserAddToGroups.svelte';
    import UserPassword from './UserPassword.svelte';
    import UserMfa from './UserMfa.svelte';
    import Devices from '$lib5/devices/Devices.svelte';
    import UserClients from '$lib/account/UserClients.svelte';
    import UserForceLogout from './UserForceLogout.svelte';
    import UserDelete from '$lib5/admin/users/UserDelete.svelte';
    import type { AuthProviderTemplate } from '$api/templates/AuthProvider';
    import { useTrigger } from '$state/callback.svelte';
    import type { UserValuesConfig } from '$api/templates/UserValuesConfig';
    import { useSession } from '$state/session.svelte';
    import AuditHistory from '$lib5/admin/audit/AuditHistory.svelte';
    import { HISTORY_TAB, isSystemUserId } from '$lib5/admin/audit/system';

    let {
        userValuesConfig,
        userId,
        providers,
        roles,
        groups,
        onSave,
        onDeleted,
    }: {
        userValuesConfig: UserValuesConfig;
        userId: string;
        providers: AuthProviderTemplate[];
        roles: RoleResponse[];
        groups: GroupResponse[];
        onSave: () => void;
        onDeleted?: () => void;
    } = $props();

    let t = useI18n();
    let ta = useI18nAdmin();

    let session = useSession('admin');
    // Group admins get a reduced tab set: profile (restricted), password, MFA reset
    // and force-logout. Attributes, devices and delete stay full-admin only, matching the
    // backend, which rejects those actions for a group admin anyway. The principal acts as a
    // delegated group admin when it reached the admin UI without the full `rauthy_admin` role.
    let isGroupAdmin = $derived(!session.isAdmin());

    const TAB_INFO = t.account.navInfo;
    const TAB_ATTR = ta.users.attributes;
    const TAB_PASSWORD = t.common.password;
    const TAB_MFA = t.account.navMfa;
    const TAB_DEVICES = t.account.devices;
    const TAB_APPS = t.account.connectedApps;
    const TAB_LOGOUT = t.account.navLogout;
    const TAB_DELETE = t.common.delete;

    // system users (101, 102, ...) are read only: their data and the history, nothing to change
    let isSystem = $derived(isSystemUserId(userId));
    let tabs = $derived(
        isSystem
            ? [TAB_INFO, HISTORY_TAB]
            : isGroupAdmin
              ? [TAB_INFO, TAB_PASSWORD, TAB_MFA, TAB_LOGOUT]
              : [
                    TAB_INFO,
                    TAB_ATTR,
                    TAB_PASSWORD,
                    TAB_MFA,
                    TAB_DEVICES,
                    TAB_APPS,
                    HISTORY_TAB,
                    TAB_LOGOUT,
                    TAB_DELETE,
                ],
    );
    let selected = $state(TAB_INFO);

    let focusFirst: undefined | (() => void) = $state();
    useTrigger().set('navSubSub', () => {
        requestAnimationFrame(() => {
            focusFirst?.();
        });
    });

    let err = $state('');
    // a group admin opened an ordinary user that is not in any of its groups yet (HTTP 428):
    // instead of an error, we offer to add the user to a managed group
    let notManaged = $state(false);
    let user: undefined | UserResponse = $state();

    $effect(() => {
        fetchUser();
    });

    async function fetchUser() {
        err = '';
        notManaged = false;
        let res = await fetchGet<UserResponse>(`/auth/v1/users/${userId}`);
        if (res.body) {
            user = undefined;
            requestAnimationFrame(() => {
                user = res.body;
            });
            focusFirst?.();
        } else if (res.status === 428) {
            user = undefined;
            notManaged = true;
        } else {
            user = undefined;
            err = res.error?.message || 'Error fetching user';
        }
    }

    async function onSaveLocal() {
        onSave();
        await fetchUser();
    }
</script>

{#if err}
    <!-- a group admin reaching a user it cannot manage (e.g. another admin) gets a clean
         notice instead of a raw API error and an empty tab bar -->
    <p class="text-muted-foreground rounded-xl border p-4 text-sm">
        {err}
    </p>
{:else if notManaged}
    <UserAddToGroups {userId} {groups} onAdded={onSaveLocal} />
{:else}
    <Tabs {tabs} bind:selected bind:focusFirst />
{/if}

{#if user && !err}
    <div class="mt-4">
        {#if selected === TAB_INFO && isSystem}
            <div class="grid gap-3 rounded-xl border p-4 text-sm">
                <p class="font-medium">System user, read only</p>
                <p class="text-muted-foreground">
                    It owns what the system does on its own, like the reminders. It cannot log in and nobody can
                    change or delete it.
                </p>
                <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
                    <dt class="text-muted-foreground">Id</dt>
                    <dd>{user.id}</dd>
                    <dt class="text-muted-foreground">Email</dt>
                    <dd>{user.email}</dd>
                    <dt class="text-muted-foreground">Name</dt>
                    <dd>{[user.given_name, user.family_name].filter(Boolean).join(' ')}</dd>
                </dl>
            </div>
        {:else if selected === HISTORY_TAB}
            <AuditHistory targetId={userId} />
        {:else if selected === TAB_INFO}
            <UserInfo
                bind:user
                config={userValuesConfig}
                {providers}
                {roles}
                {groups}
                {isGroupAdmin}
                onSave={onSaveLocal}
            />
        {:else if selected === TAB_ATTR}
            <UserAttr {user} {onSave} />
        {:else if selected === TAB_PASSWORD}
            <UserPassword {user} {onSave} />
        {:else if selected === TAB_MFA}
            <UserMfa {user} {onSave} />
        {:else if selected === TAB_DEVICES}
            <Devices {userId} />
        {:else if selected === TAB_APPS}
            <UserClients {userId} />
        {:else if selected === TAB_LOGOUT}
            <UserForceLogout {userId} />
        {:else if selected === TAB_DELETE}
            <UserDelete {userId} {onSave} {onDeleted} />
        {/if}
    </div>
{/if}
