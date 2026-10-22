<script lang="ts">
    import Msg from '$lib5/Msg.svelte';
    import Button from '$lib5/button/Button.svelte';
    import IconCheck from '$icons/IconCheck.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { fetchDelete } from '$api/fetch';

    let {
        userId,
    }: {
        userId: string;
    } = $props();

    let ta = useI18nAdmin();

    let err = $state('');
    let success = $state(false);

    async function onSubmit() {
        err = '';

        let res = await fetchDelete(`/auth/v1/sessions/${userId}`);
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

<div class="grid max-w-xl gap-4">
    <p class="text-muted-foreground text-sm">{ta.users.forceLogout}</p>

    <div class="flex items-center gap-3">
        <Button level={-1} onclick={onSubmit}>Logout</Button>
        <Msg {success} {err} />
    </div>
</div>
