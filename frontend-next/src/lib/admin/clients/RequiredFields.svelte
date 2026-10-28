<script lang="ts">
    import FormSection from '$lib5/FormSection.svelte';
    import { Checkbox } from '$lib/components/ui/checkbox';
    import { Label } from '$lib/components/ui/label';
    import Tooltip from '$lib5/Tooltip.svelte';
    import { fetchGet } from '$api/fetch';
    import type { SettingsResponse } from '$api/types/settings';
    import type { ScopeResponse } from '$api/types/scopes';
    import { onMount } from 'svelte';

    // The user data a client needs before a login for it may finish. Only the data the selected
    // scopes release can be picked; users who lack some of it are sent to their account page.
    let {
        scopes,
        required = $bindable(),
    }: {
        /** names of the scopes this client can request (allowed + default) */
        scopes: string[];
        required: string[];
    } = $props();

    const STANDARD: Record<string, [string, string][]> = {
        profile: [
            ['given_name', 'Given name'],
            ['family_name', 'Family name'],
            ['preferred_username', 'Username'],
            ['birthdate', 'Birthdate'],
            ['gender', 'Gender'],
            ['tz', 'Time zone'],
        ],
        email: [['email_verified', 'Verified email address']],
        phone: [['phone', 'Phone number']],
        address: [
            ['street', 'Street'],
            ['zip', 'ZIP code'],
            ['city', 'City'],
            ['country', 'Country'],
        ],
    };

    let custom: ScopeResponse[] = $state([]);
    /** profile fields the admin hid for all users: they cannot fill these in themselves */
    let hidden: Set<string> = $state(new Set());

    onMount(async () => {
        let res = await fetchGet<ScopeResponse[]>('/auth/v1/scopes');
        if (res.body) {
            custom = res.body;
        }
        let settings = await fetchGet<SettingsResponse>('/auth/v1/settings');
        if (settings.body) {
            hidden = new Set(
                Object.entries(settings.body.user_values)
                    .filter(([, v]) => v === 'hidden')
                    .map(([k]) => k),
            );
        }
    });

    let groups = $derived.by(() => {
        let out: { scope: string; fields: [string, string][] }[] = [];
        for (let scope of scopes) {
            if (STANDARD[scope]) {
                out.push({ scope, fields: STANDARD[scope] });
                continue;
            }
            let c = custom.find(s => s.name === scope);
            let names = [
                ...new Set([...(c?.attr_include_access ?? []), ...(c?.attr_include_id ?? [])]),
            ];
            if (names.length > 0) {
                out.push({ scope, fields: names.map(n => [`attr:${n}`, n]) });
            }
        }
        return out;
    });

    /** keys that can currently be picked; the parent drops the rest when saving */
    export function visibleKeys(): Set<string> {
        return new Set(groups.flatMap(g => g.fields.map(f => f[0])));
    }

    function toggle(key: string, on: boolean) {
        required = on ? [...new Set([...required, key])] : required.filter(k => k !== key);
    }
</script>

<FormSection
    title="Required user data"
    desc="Tick what a user must have filled in to log in to this client. If something is missing, the user is asked to complete it on their account page first. Only data released by the scopes above is listed."
>
    {#if groups.length === 0}
        <p class="text-muted-foreground text-sm">
            None of the enabled scopes release user data that can be required.
        </p>
    {/if}
    {#each groups as g (g.scope)}
        <div class="grid gap-2">
            <span class="text-sm font-medium">
                <code class="bg-muted rounded px-1.5 py-0.5 text-xs">{g.scope}</code>
            </span>
            <div class="grid gap-2 sm:grid-cols-2">
                {#each g.fields as [key, label] (key)}
                    <Label class="inline-flex items-center gap-2 font-normal">
                        <Checkbox
                            checked={required.includes(key)}
                            onCheckedChange={v => toggle(key, v === true)}
                            aria-label={label}
                        />
                        <span>{label}</span>
                        {#if hidden.has(key)}
                            <Tooltip
                                text={`Users cannot see or fill in this field, so a client that requires it would lock them out. To show it, open Config -> User fields and set "${label}" to Optional or Required.`}
                            >
                                <span class="text-muted-foreground cursor-help">(hidden)</span>
                            </Tooltip>
                        {/if}
                    </Label>
                {/each}
            </div>
        </div>
    {/each}
</FormSection>
