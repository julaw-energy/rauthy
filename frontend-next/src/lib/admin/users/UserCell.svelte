<script lang="ts">
    import UserPicture from '$lib/UserPicture.svelte';
    import type { UserResponseSimple } from '$api/types/user.ts';

    let { row }: { row: UserResponseSimple } = $props();

    function fallbackCharacters(user: UserResponseSimple) {
        let chars = '';
        if (user.given_name) {
            chars = user.given_name[0];
        } else {
            chars = user.email[0];
        }
        if (user.family_name) {
            chars += user.family_name[0];
        }
        return chars;
    }
</script>

<div class="flex min-w-0 items-center gap-3">
    <UserPicture
        fallbackCharacters={fallbackCharacters(row)}
        userId={row.id}
        pictureId={row.picture_id}
        size="small"
        disableUpload
    />
    <div class="flex min-w-0 flex-col leading-tight">
        <span class="truncate">{row.email}</span>
        <span class="text-muted-foreground truncate text-xs">{row.given_name ?? ''} {row.family_name ?? ''}</span>
    </div>
</div>
