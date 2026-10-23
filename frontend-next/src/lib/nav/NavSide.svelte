<script lang="ts">
    import * as Sidebar from '$lib/components/ui/sidebar';
    import { Separator } from '$lib/components/ui/separator';
    import AppVersion from '$lib5/AppVersion.svelte';
    import { brand } from '$lib5/appName.svelte';
    import RauthyLogo from '$lib5/RauthyLogo.svelte';
    import NavLink from '$lib5/nav/NavLink.svelte';
    import IconUserGroup from '$icons/IconUserGroup.svelte';
    import IconDocText from '$icons/IconDocText.svelte';
    import IconOffice from '$icons/IconOffice.svelte';
    import IconShieldCheck from '$icons/IconShieldCheck.svelte';
    import IconUser from '$icons/IconUser.svelte';
    import IconId from '$icons/IconId.svelte';
    import IconBell from '$icons/IconBell.svelte';
    import IconStop from '$icons/IconStop.svelte';
    import IconKey from '$icons/IconKey.svelte';
    import IconCloud from '$icons/IconCloud.svelte';
    import IconWrenchScrew from '$icons/IconWrenchScrew.svelte';
    import IconBookOpen from '$icons/IconBookOpen.svelte';
    import IconCheckBadge from '$icons/IconCheckBadge.svelte';
    import IconCommandLine from '$icons/IconCommandLine.svelte';
    import IconCircleStack from '$icons/IconCircleStack.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useSession } from '$state/session.svelte';
    import { page } from '$app/state';

    let ta = useI18nAdmin();
    let session = useSession('admin');

    // Delegated group admins only get the user-centric, read-only sections.
    // Everything that manages clients, roles, groups, scopes, system config, etc. stays
    // full-admin only. The backend enforces this regardless; here we just hide it.
    let isGroupAdmin = $derived(!session.isAdmin());

    let params = $state('');
    // cannot be derived because of static pre-rendering
    $effect(() => {
        params = page.url.search;
    });
</script>

<Sidebar.Root collapsible="icon">
    <Sidebar.Header>
        <div class="flex items-center gap-2 px-1 group-data-[collapsible=icon]:justify-center">
            <RauthyLogo width="2rem" />
            <span class="text-lg font-semibold tracking-tight group-data-[collapsible=icon]:hidden">{brand.name}</span>
        </div>
    </Sidebar.Header>

    <Sidebar.Content>
        <Sidebar.Group>
            <Sidebar.GroupContent>
                <Sidebar.Menu>
                    <NavLink {params} route="/users">
                        {#snippet icon(width: string)}<IconUser {width} />{/snippet}
                        {ta.nav.users}
                    </NavLink>

                    {#if !isGroupAdmin}
                        <NavLink {params} route="/clients">
                            {#snippet icon(width: string)}<IconOffice {width} />{/snippet}
                            {ta.nav.clients}
                        </NavLink>
                        <NavLink {params} route="/roles">
                            {#snippet icon(width: string)}<IconCheckBadge {width} />{/snippet}
                            {ta.nav.roles}
                        </NavLink>
                        <NavLink {params} route="/groups">
                            {#snippet icon(width: string)}<IconUserGroup {width} />{/snippet}
                            {ta.nav.groups}
                        </NavLink>
                        <NavLink {params} route="/attributes">
                            {#snippet icon(width: string)}<IconDocText {width} />{/snippet}
                            {ta.nav.attributes}
                        </NavLink>
                        <NavLink {params} route="/scopes">
                            {#snippet icon(width: string)}<IconId {width} />{/snippet}
                            {ta.nav.scopes}
                        </NavLink>
                        <NavLink {params} route="/pam">
                            {#snippet icon(width: string)}<IconCommandLine {width} />{/snippet}
                            PAM
                        </NavLink>
                    {/if}

                    <NavLink {params} route="/sessions">
                        {#snippet icon(width: string)}<IconShieldCheck {width} />{/snippet}
                        {ta.nav.sessions}
                    </NavLink>
                    <NavLink {params} route="/events">
                        {#snippet icon(width: string)}<IconBell {width} />{/snippet}
                        {ta.nav.events}
                    </NavLink>
                    <NavLink {params} route="/blacklist">
                        {#snippet icon(width: string)}<IconStop {width} color="currentColor" />{/snippet}
                        {ta.nav.blacklist}
                    </NavLink>

                    {#if !isGroupAdmin}
                        <NavLink {params} route="/api_keys">
                            {#snippet icon(width: string)}<IconKey {width} />{/snippet}
                            {ta.nav.apiKeys}
                        </NavLink>
                        <NavLink {params} route="/providers">
                            {#snippet icon(width: string)}<IconCloud {width} />{/snippet}
                            {ta.nav.providers}
                        </NavLink>
                        <NavLink {params} route="/kv">
                            {#snippet icon(width: string)}<IconCircleStack {width} />{/snippet}
                            KV Store
                        </NavLink>
                        <NavLink {params} route="/config/policy" highlightIncludes="/config/">
                            {#snippet icon(width: string)}<IconWrenchScrew {width} />{/snippet}
                            {ta.nav.config}
                        </NavLink>
                    {/if}

                    <NavLink {params} route="/docs">
                        {#snippet icon(width: string)}<IconBookOpen {width} />{/snippet}
                        {ta.nav.docs}
                    </NavLink>
                </Sidebar.Menu>
            </Sidebar.GroupContent>
        </Sidebar.Group>
    </Sidebar.Content>

    <Sidebar.Footer>
        <Separator />
        <!-- date format, theme, account link and logout live in the page header (routes/admin/+layout) -->
        <div class="group-data-[collapsible=icon]:hidden">
            <div class="px-2"><AppVersion /></div>
        </div>
    </Sidebar.Footer>
    <Sidebar.Rail />
</Sidebar.Root>
