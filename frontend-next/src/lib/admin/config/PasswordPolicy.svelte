<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import { onMount } from 'svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import { fetchGet, fetchPut } from '$api/fetch';
    import type {
        PasswordPolicyRequest,
        PasswordPolicyResponse,
    } from '$api/types/password_policy.ts';
    import Form from '$lib5/form/Form.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import IconCheck from '$icons/IconCheck.svelte';

    const inputWidth = '14rem';

    let t = useI18n();
    let ta = useI18nAdmin();

    let err = $state('');
    let success = $state(false);
    let policy: undefined | PasswordPolicyResponse = $state();

    onMount(async () => {
        let res = await fetchGet<PasswordPolicyResponse>('/auth/v1/password_policy');
        if (res.body) {
            let p = res.body;
            if (!p.include_lower_case) {
                p.include_lower_case = 0;
            }
            if (!p.include_upper_case) {
                p.include_upper_case = 0;
            }
            if (!p.include_digits) {
                p.include_digits = 0;
            }
            if (!p.include_special) {
                p.include_special = 0;
            }
            if (!p.not_recently_used) {
                p.not_recently_used = 0;
            }
            if (!p.valid_days) {
                p.valid_days = 0;
            }
            policy = p;
        } else {
            console.error(res.error);
        }
    });

    async function onSubmit(form: HTMLFormElement, params: URLSearchParams) {
        err = '';

        if (!policy) {
            return;
        }
        if (policy.length_max < policy.length_min) {
            err = 'Max Length cannot be lower than Min Length';
            return false;
        }
        let sum =
            (policy.include_digits || 0) +
            (policy.include_lower_case || 0) +
            (policy.include_upper_case || 0) +
            (policy.include_special || 0);
        if (sum > policy.length_max) {
            err = 'The sum of all includes does not fit into Max Length';
            return false;
        }

        let payload: PasswordPolicyRequest = {
            length_min: policy.length_min,
            length_max: policy.length_max,
            not_recently_used: policy.not_recently_used ? policy.not_recently_used : undefined,
            valid_days: policy.valid_days ? policy.valid_days : undefined,
            include_digits: policy.include_digits ? policy.include_digits : undefined,
            include_lower_case: policy.include_lower_case ? policy.include_lower_case : undefined,
            include_upper_case: policy.include_upper_case ? policy.include_upper_case : undefined,
            include_special: policy.include_special ? policy.include_special : undefined,
        };

        let res = await fetchPut(form.action, payload);
        if (res.error) {
            err = res.error.message;
        } else {
            success = true;
            setTimeout(() => {
                success = false;
            }, 3000);
        }
    }
</script>

<div class="grid max-w-3xl gap-4">
    <h2 class="text-xl font-semibold">{t.passwordPolicy.passwordPolicy}</h2>
    <p class="text-muted-foreground text-sm">
        {ta.passwordPolicy.configDesc}
    </p>

    <Form action="/auth/v1/password_policy" {onSubmit}>
        {#if policy}
            <div class="grid gap-4 sm:grid-cols-2">
                <Input
                    typ="number"
                    bind:value={policy.length_min}
                    label={t.passwordPolicy.lengthMin}
                    placeholder={t.passwordPolicy.lengthMin}
                    autocomplete="off"
                    min="8"
                    max="128"
                    required
                />
                <Input
                    typ="number"
                    bind:value={policy.length_max}
                    label={t.passwordPolicy.lengthMax}
                    placeholder={t.passwordPolicy.lengthMax}
                    autocomplete="off"
                    min="8"
                    max="128"
                    required
                />
                <Input
                    typ="number"
                    bind:value={policy.include_lower_case}
                    label={t.passwordPolicy.lowercaseMin}
                    placeholder={t.passwordPolicy.lowercaseMin}
                    autocomplete="off"
                    min="0"
                    max="32"
                />
                <Input
                    typ="number"
                    bind:value={policy.include_upper_case}
                    label={t.passwordPolicy.uppercaseMin}
                    placeholder={t.passwordPolicy.uppercaseMin}
                    autocomplete="off"
                    min="0"
                    max="32"
                />
                <Input
                    typ="number"
                    bind:value={policy.include_digits}
                    label={t.passwordPolicy.digitsMin}
                    placeholder={t.passwordPolicy.digitsMin}
                    autocomplete="off"
                    min="0"
                    max="32"
                />
                <Input
                    typ="number"
                    bind:value={policy.include_special}
                    label={t.passwordPolicy.specialMin}
                    placeholder={t.passwordPolicy.specialMin}
                    autocomplete="off"
                    min="0"
                    max="32"
                />
                <Input
                    typ="number"
                    bind:value={policy.not_recently_used}
                    label={t.passwordPolicy.notRecent}
                    placeholder={t.passwordPolicy.notRecent}
                    autocomplete="off"
                    min="0"
                    max="32"
                />
            </div>

            <p class="text-muted-foreground text-sm">
                {ta.passwordPolicy.validityNew}<br />
                {ta.passwordPolicy.resetSet0}
            </p>

            <div class="sm:max-w-xs">
                <Input
                    typ="number"
                    bind:value={policy.valid_days}
                    label={ta.passwordPolicy.validForDays}
                    placeholder={ta.passwordPolicy.validForDays}
                    autocomplete="off"
                    min="0"
                    max="3650"
                />
            </div>

            <div class="flex items-center gap-3">
                <Button type="submit">
                    {t.common.save}
                </Button>
                <Msg {success} {err} />
            </div>
        {/if}
    </Form>
</div>
