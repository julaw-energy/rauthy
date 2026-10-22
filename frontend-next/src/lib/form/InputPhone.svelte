<script lang="ts">
    import type { FullAutoFill } from 'svelte/elements';
    import { Check, ChevronDown } from '@lucide/svelte';
    import {
        AsYouType,
        getCountries,
        getCountryCallingCode,
        parsePhoneNumberFromString,
        type CountryCode,
    } from 'libphonenumber-js/min';
    import { countryFromBrowser } from '$lib5/countries';
    import * as Popover from '$lib/components/ui/popover';
    import * as Command from '$lib/components/ui/command';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { cn } from '$lib/utils';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey } from '$utils/helpers';

    // Value is always E.164 (`+254712345678`), which is what the backend expects (PATTERN_PHONE).
    let {
        id = genKey(),
        name,
        value = $bindable(),
        label = '',
        placeholder = '',
        disabled = false,
        required = false,
        errMsg,
        autocomplete = 'tel-national',
        country = $bindable(),
        onEnter,
    }: {
        id?: string;
        name?: string;
        value?: string | null;
        label?: string;
        placeholder?: string;
        disabled?: boolean;
        required?: boolean;
        errMsg?: string;
        autocomplete?: FullAutoFill;
        country?: CountryCode;
        onEnter?: () => void;
    } = $props();

    let t = useI18n();

    const names = (() => {
        try {
            return new Intl.DisplayNames([navigator.language || 'en'], { type: 'region' });
        } catch {
            return undefined;
        }
    })();

    const flag = (c: string) =>
        String.fromCodePoint(...[...c.toUpperCase()].map(ch => 127397 + ch.charCodeAt(0)));

    const countries = getCountries()
        .map(c => ({ code: c, name: names?.of(c) ?? c, dial: getCountryCallingCode(c) }))
        .sort((a, b) => a.name.localeCompare(b.name));

    function guessCountry(): CountryCode {
        // the browser timezone first (language alone makes e.g. `en-GB` a +44 default everywhere)
        const fromTz = countryFromBrowser();
        if (fromTz && getCountries().includes(fromTz as CountryCode)) return fromTz as CountryCode;
        try {
            const region = new Intl.Locale(navigator.language).maximize().region;
            if (region && getCountries().includes(region as CountryCode)) return region as CountryCode;
        } catch {
            // fall through
        }
        return 'US';
    }

    // `country` is bindable so a parent can preselect it (e.g. after the country field changed);
    // a bindable prop must not have a fallback, hence the separate internal state
    let own: CountryCode = $state(guessCountry());
    $effect(() => {
        if (country && country !== own) own = country;
    });
    $effect(() => {
        country = own;
    });
    let national = $state('');
    let open = $state(false);
    let isError = $state(false);
    let input: HTMLInputElement | null = $state(null);

    // sync in: external value -> country + national number (only when it differs from what we emit)
    let lastEmitted: string | null | undefined = undefined;
    $effect(() => {
        const v = value ?? '';
        if (v === (lastEmitted ?? '')) return;
        const parsed = v ? parsePhoneNumberFromString(v) : undefined;
        if (parsed?.country) {
            own = parsed.country;
            national = new AsYouType(parsed.country).input(parsed.nationalNumber);
        } else if (v) {
            national = v.replace(/^\+/, '');
        } else {
            national = '';
        }
        lastEmitted = v;
    });

    function emit() {
        const digits = national.replace(/[^\d]/g, '');
        // parsing drops a national trunk prefix typed by the user (`0700...` -> `+256700...`)
        const next = digits
            ? (parsePhoneNumberFromString(digits, own)?.number ?? `+${getCountryCallingCode(own)}${digits}`)
            : '';
        lastEmitted = next;
        value = next;
        const parsed = next ? parsePhoneNumberFromString(next, own) : undefined;
        input?.setCustomValidity(next && !parsed?.isPossible() ? (errMsg || t.common.invalidInput) : '');
    }

    function onInput(ev: Event & { currentTarget: HTMLInputElement }) {
        isError = false;
        // a pasted international number selects its country
        const raw = ev.currentTarget.value;
        if (raw.trim().startsWith('+')) {
            const parsed = parsePhoneNumberFromString(raw);
            if (parsed?.country) {
                own = parsed.country;
                national = new AsYouType(parsed.country).input(parsed.nationalNumber);
                emit();
                return;
            }
        }
        national = new AsYouType(own).input(raw);
        emit();
    }

    function pick(c: CountryCode) {
        own = c;
        open = false;
        national = national ? new AsYouType(c).input(national.replace(/[^\d]/g, '')) : '';
        emit();
        input?.focus();
    }

    let current = $derived(countries.find(c => c.code === own));
</script>

<div class="grid w-full gap-1.5">
    {#if label}
        <Label for={id}>{label}{#if required}<span class="text-destructive">*</span>{/if}</Label>
    {/if}
    <input type="hidden" {name} value={value ?? ''} />
    <div class="flex gap-1.5">
        <Popover.Root bind:open>
            <Popover.Trigger
                {disabled}
                aria-label="Country"
                class={cn(
                    'border-input bg-white dark:bg-input/50 dark:border-transparent flex h-9 shrink-0 items-center gap-1.5 rounded-3xl border px-3 text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/30 disabled:opacity-50',
                )}
            >
                <span class="text-base leading-none">{flag(own)}</span>
                <span class="text-muted-foreground tabular-nums">+{current?.dial}</span>
                <ChevronDown class="text-muted-foreground size-4" />
            </Popover.Trigger>
            <Popover.Content class="w-72 p-0" align="start">
                <Command.Root>
                    <Command.Input placeholder="Search country or code" />
                    <Command.List class="max-h-64">
                        <Command.Empty>-</Command.Empty>
                        <Command.Group>
                            {#each countries as c (c.code)}
                                <Command.Item
                                    value="{c.name} {c.code} +{c.dial}"
                                    onSelect={() => pick(c.code)}
                                >
                                    <Check class={cn('size-4', c.code !== own && 'opacity-0')} />
                                    <span class="text-base leading-none">{flag(c.code)}</span>
                                    <span class="min-w-0 flex-1 truncate">{c.name}</span>
                                    <span class="text-muted-foreground tabular-nums">+{c.dial}</span>
                                </Command.Item>
                            {/each}
                        </Command.Group>
                    </Command.List>
                </Command.Root>
            </Popover.Content>
        </Popover.Root>

        <Input
            {id}
            bind:ref={input}
            type="tel"
            inputmode="tel"
            {autocomplete}
            {placeholder}
            {disabled}
            {required}
            maxlength={32}
            value={national}
            aria-invalid={isError}
            oninput={onInput}
            oninvalid={() => (isError = true)}
            onkeydown={(ev: KeyboardEvent) => ev.code === 'Enter' && onEnter?.()}
            class="min-w-0 flex-1"
        />
    </div>
    {#if isError}
        <p class="text-destructive text-xs">{errMsg || t.common.invalidInput}</p>
    {/if}
</div>
