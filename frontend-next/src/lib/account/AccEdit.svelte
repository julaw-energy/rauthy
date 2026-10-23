<script lang="ts">
    import InputGender from '$lib5/form/InputGender.svelte';
    import { tzAfterCountryChange, resolveCountry, countryFromBrowser, browserTimezone } from '$lib5/countries';
    import type { CountryCode } from 'libphonenumber-js/min';
    import InputCountry from '$lib5/form/InputCountry.svelte';
    import InputPhone from '$lib5/form/InputPhone.svelte';
    import ContactVerify from '$lib5/form/ContactVerify.svelte';
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import { fade } from 'svelte/transition';
    import Input from '$lib5/form/Input.svelte';
    import { useI18n } from '$state/i18n.svelte.js';
    import type { UpdateUserSelfRequest, UserResponse } from '$api/types/user.ts';
    import Form from '$lib5/form/Form.svelte';
    import {
        PATTERN_ALNUM,
        PATTERN_CITY,
        PATTERN_PHONE,
        PATTERN_STREET,
        PATTERN_USER_NAME,
    } from '$utils/patterns';
    import IconCheck from '$icons/IconCheck.svelte';
    import { fetchDelete, fetchGet, fetchPut } from '$api/fetch';
    import InputDateTimeCombo from '$lib5/form/InputDateTimeCombo.svelte';
    import type { UserValuesConfig } from '$api/templates/UserValuesConfig';
    import { onMount } from 'svelte';
    import TZSelect from '$lib/TZSelect.svelte';
    import PreferredUsername from '$lib/PreferredUsername.svelte';

    let {
        config,
        user = $bindable(),
        viewModePhone,
    }: {
        config: undefined | UserValuesConfig;
        user: UserResponse;
        viewModePhone?: boolean;
    } = $props();

    if (!user.user_values.birthdate) {
        user.user_values.birthdate = '';
    }
    if (!user.user_values.preferred_username) {
        user.user_values.preferred_username = '';
    }
    if (!user.user_values.tz) {
        user.user_values.tz = 'UTC';
    }

    let t = useI18n();

    let err = $state('');
    let success = $state(false);
    let successEmailConfirm = $state(false);

    let canSelfDelete = $state(false);
    let showDeleteConfirm = $state(false);
    let deleteConfirmValue = $state('');

    onMount(() => {
        fetchSelfDeleteConfig();
    });

    $effect(() => {
        if (showDeleteConfirm) {
            deleteConfirmValue = '';
        }
    });

    async function fetchSelfDeleteConfig() {
        let res = await fetchGet(`/auth/v1/users/${user.id}/self/delete`);
        canSelfDelete = res.status === 202;
    }

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        if (!config) {
            return;
        }

        const email = params.get('email');
        const familyName = params.get('family_name') || undefined;
        const givenName = params.get('given_name') || undefined;

        let userValues = {
            birthdate: params.get('birthdate') || undefined,
            gender: params.get('gender') || undefined,
            phone: params.get('phone')?.replaceAll(' ', '') || undefined,
            street: params.get('street') || undefined,
            zip: params.get('zip') || undefined,
            city: params.get('city') || undefined,
            country: params.get('country') || undefined,
            tz: user.user_values.tz || undefined,
        };
        if (userValues.tz === 'Etc/UTC' || userValues.tz === 'UTC') {
            userValues.tz = undefined;
        }

        let payload: UpdateUserSelfRequest = {
            email,
            family_name: familyName,
            given_name: givenName,
            user_values: userValues,
        };

        let res = await fetchPut<UserResponse>(`/auth/v1/users/${user.id}/self`, payload);
        if (res.body) {
            success = true;
            if (!res.body.user_values?.birthdate) {
                res.body.user_values.birthdate = '';
            }
            if (!res.body.user_values?.tz) {
                res.body.user_values.tz = 'UTC';
            }
            user = res.body;

            if (!user.user_values.birthdate) {
                user.user_values.birthdate = '';
            }
            if (!user.user_values.preferred_username) {
                user.user_values.preferred_username = '';
            }
            if (!user.user_values.tz) {
                user.user_values.tz = 'UTC';
            }

            let timer = 3000;
            if (res.status === 202) {
                successEmailConfirm = true;
                timer = 30000;
            }

            setTimeout(() => {
                success = false;
            }, timer);
        } else if (res.error) {
            console.error(res.error);
            err = res.error.message;
        }
    }

    async function onSubmitDelete() {
        let res = await fetchDelete(`/auth/v1/users/${user.id}/self/delete`);
        if (res.status === 204) {
            window.location.href = '/auth/v1/';
        }
    }

    let phoneCountry: CountryCode | undefined = $state();
    let prevCountry: string | undefined = resolveCountry(user.user_values.country);

    // own account only: when no country is stored yet, preselect it from the browser timezone
    // (stays editable, never overrides a saved value)
    if (config?.country !== 'hidden' && !user.user_values.country) {
        const guess = countryFromBrowser();
        if (guess) {
            user.user_values.country = guess;
            prevCountry = guess;
            if (!user.user_values.phone) phoneCountry = guess as CountryCode;
            const btz = browserTimezone();
            if (btz && (!user.user_values.tz || user.user_values.tz === 'UTC' || user.user_values.tz === 'Etc/UTC')) {
                user.user_values.tz = btz;
            }
        }
    }

    // picking a country pre-fills what depends on it: the timezone and the phone dial code
    function onCountryChange(code: string) {
        const tz = tzAfterCountryChange(user.user_values.tz, prevCountry, code);
        if (tz) user.user_values.tz = tz;
        if (!user.user_values.phone) phoneCountry = code as CountryCode;
        prevCountry = code;
    }
</script>

<div class="grid max-w-3xl gap-6">
    {#if config}
        <Form action={`/auth/v1/users/${user.id}/self`} {onSubmit}>
            <div class="grid gap-4 md:grid-cols-2">
                <div class="grid content-start gap-4">
                    <Input
                        typ="email"
                        name="email"
                        label={t.common.email}
                        placeholder={t.common.email}
                        value={user.email}
                        required
                    />
                    <ContactVerify userId={user.id} kind="email" current={user.email} />
                    {#if config.given_name !== 'hidden'}
                        <Input
                            name="given_name"
                            autocomplete="given-name"
                            label={t.account.givenName}
                            placeholder={t.account.givenName}
                            value={user.given_name}
                            required={config.given_name === 'required'}
                            maxLength={32}
                            pattern={PATTERN_USER_NAME}
                        />
                    {/if}
                    {#if config.family_name !== 'hidden'}
                        <Input
                            name="family_name"
                            autocomplete="family-name"
                            label={t.account.familyName}
                            placeholder={t.account.familyName}
                            value={user.family_name}
                            required={config.family_name === 'required'}
                            maxLength={32}
                            pattern={PATTERN_USER_NAME}
                        />
                    {/if}
                    {#if config.birthdate !== 'hidden'}
                        <InputDateTimeCombo
                            name="birthdate"
                            label={t.account.birthdate}
                            bind:value={user.user_values.birthdate}
                            required={config.birthdate === 'required'}
                            withDelete
                            openTop
                        />
                    {/if}
                    {#if config.gender !== 'hidden'}
                        <InputGender
                            name="gender"
                            label={t.account.gender}
                            bind:value={user.user_values.gender}
                            required={config.gender === 'required'}
                        />
                    {/if}
                    <PreferredUsername
                        userId={user.id}
                        bind:preferred_username={user.user_values.preferred_username}
                        config={config.preferred_username}
                    />
                </div>
                <div class="grid content-start gap-4">
                    {#if config.country !== 'hidden'}
                        <InputCountry
                            name="country"
                            label={t.account.country}
                            placeholder={t.account.country}
                            value={user.user_values.country}
                            required={config.country === 'required'}
                            onChange={onCountryChange}
                        />
                    {/if}
                    {#if config.tz !== 'hidden'}
                        <TZSelect bind:value={user.user_values.tz} />
                    {/if}
                    {#if config.phone !== 'hidden'}
                        <InputPhone
                            name="phone"
                            label={t.account.phone}
                            placeholder={t.account.phone}
                            bind:value={user.user_values.phone}
                            bind:country={phoneCountry}
                            required={config.phone === 'required'}
                        />
                        <ContactVerify userId={user.id} kind="phone" current={user.user_values.phone} />
                    {/if}
                    {#if config.city !== 'hidden'}
                        <Input
                            name="city"
                            autocomplete="address-level2"
                            label={t.account.city}
                            placeholder={t.account.city}
                            value={user.user_values.city}
                            required={config.city === 'required'}
                            maxLength={48}
                            pattern={PATTERN_CITY}
                        />
                    {/if}
                    {#if config.street !== 'hidden'}
                        <Input
                            name="street"
                            autocomplete="street-address"
                            label={t.account.street}
                            placeholder={t.account.street}
                            value={user.user_values.street}
                            required={config.street === 'required'}
                            maxLength={48}
                            pattern={PATTERN_STREET}
                        />
                    {/if}
                    {#if config.zip !== 'hidden'}
                        <Input
                            name="zip"
                            autocomplete="postal-code"
                            label={t.account.zip}
                            placeholder={t.account.zip}
                            value={user.user_values.zip}
                            required={config.zip === 'required'}
                            maxLength={24}
                            pattern={PATTERN_ALNUM}
                        />
                    {/if}
                </div>
            </div>

            <div class="flex flex-wrap items-center gap-2">
                <Button type="submit">
                    {t.common.save}
                </Button>
                {#if canSelfDelete}
                    <Button level={-3} onclick={() => (showDeleteConfirm = !showDeleteConfirm)}>
                        {t.account.deleteAccount.deleteAccount}
                    </Button>
                {/if}
                <Msg {success} />
            </div>
            {#if successEmailConfirm}
                <p class="text-sm">{t.account.emailUpdateConfirm}</p>
            {:else}
                <Msg {err} />
            {/if}
        </Form>
    {/if}

    {#if showDeleteConfirm}
        <div class="border-destructive/40 grid gap-3 rounded-xl border p-4">
            <h5 class="text-destructive font-semibold">{t.account.deleteAccount.deleteAccount}</h5>
            <p class="text-sm">
                {t.account.deleteAccount.deleteAccountDesc}
                <b>{t.common.delete.toUpperCase()}</b>
            </p>

            <Input
                label={t.common.delete.toUpperCase()}
                placeholder={t.common.delete.toUpperCase()}
                bind:value={deleteConfirmValue}
                width="15rem"
            />
            <div>
                <Button
                    level={deleteConfirmValue !== t.common.delete.toUpperCase() ? -3 : -1}
                    onclick={onSubmitDelete}
                    isDisabled={deleteConfirmValue !== t.common.delete.toUpperCase()}
                >
                    {t.common.delete}
                </Button>
            </div>
        </div>
    {/if}
</div>
