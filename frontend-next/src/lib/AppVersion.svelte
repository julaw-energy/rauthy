<script lang="ts">
    import { onMount } from 'svelte';
    import { fetchGet } from '$api/fetch';
    import type { AppVersionResponse } from '$api/types/api_version.ts';

    let version: undefined | AppVersionResponse = $state();

    onMount(async () => {
        let res = await fetchGet<AppVersionResponse>('/auth/v1/version');
        if (res.body) {
            version = res.body;
        } else {
            console.error(res.error);
        }
    });
</script>

{#if version}
    <div class="text-muted-foreground text-xs">
        {#if version.update_available}
            <a class="hover:underline" href={version.latest_url} target="_blank" rel="noopener noreferrer">
                v{version.current} ⚠️
            </a>
        {:else}
            v{version.current}
        {/if}
    </div>
{/if}
