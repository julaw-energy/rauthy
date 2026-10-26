<script lang="ts">
    import IconUpload from '$icons/IconUpload.svelte';
    import { useI18n } from '$state/i18n.svelte';
    import { genKey, getCsrfToken } from '$utils/helpers';
    import type { ErrorResponse } from '$api/types/error.ts';
    import IconStop from '$icons/IconStop.svelte';
    import Button from '$lib/button/Button.svelte';
    import { fetchDelete } from '$api/fetch';
    import { usePictureConfig } from '$state/picture_config.svelte';

    let {
        userId,
        fallbackCharacters,
        // image_id will be appended to the path if given, and
        // its existence will be checked in the upload response
        pictureId = $bindable(),
        size = 'medium',
        disableUpload,
    }: {
        userId?: string;
        fallbackCharacters: string | undefined;
        pictureId?: string;
        size?: 'small' | 'medium' | 'large';
        disableUpload?: boolean;
    } = $props();

    const id = genKey();
    const accept = 'image/png, image/jpeg, image/webp, image/svg+xml';

    let t = useI18n();

    let refDel: undefined | HTMLButtonElement = $state();

    let config = $derived(usePictureConfig().get());

    let err = $state('');
    let errFileSize = $state('');
    let timer: undefined | number;

    let showUpload = $state(false);
    let isUploading = $state(false);
    let files: undefined | FileList = $state();

    // A federated user starts with the picture link of the provider instead of one of our ids. It is
    // shown from there, and an upload replaces it.
    let isExternal = $derived(!!pictureId && pictureId.startsWith('https://'));

    let color = $derived(calculateColor());
    let width = $derived.by(() => {
        switch (size) {
            case 'small':
                return '2.25rem';
            case 'medium':
                return '3rem';
            default:
                // max size of the picture saved in the background
                return '192px';
        }
    });

    $effect(() => {
        if (err) {
            console.error(err);
        }
    });

    $effect(() => {
        if (errFileSize) {
            if (timer) {
                clearTimeout(timer);
            }
            timer = window.setTimeout(() => {
                errFileSize = '';
            }, 5000);
        }
    });

    $effect(() => {
        if (files && files.length > 0) {
            upload(files);
        }
    });

    function calculateColor() {
        let hue = 0;
        if (fallbackCharacters) {
            hue = stringToHue(fallbackCharacters);
        }
        return `hsl(${hue}, 50%, 50%)`;
    }

    function ondrop(ev: DragEvent) {
        files = ev.dataTransfer?.files;
    }

    function onerror(ev: Event & { currentTarget: EventTarget & Element }) {
        if (ev.type === 'error') {
            err = 'Input Error';
        }
    }

    function onmouseenter() {
        showUpload = true;
    }

    function onmouseleave() {
        showUpload = false;
    }

    /// Converts the given string to a random hue.
    /// Will always return the same result for the same input.
    /// Ignores any characters after the first 5 for efficiency.
    function stringToHue(s: string) {
        let hues = [1, 255, 3, 13, 19];

        for (let i = 0; i < 5; i++) {
            let code = s.charCodeAt(i);
            if (code) {
                hues[i] = code;
            } else {
                break;
            }
        }

        return (hues[0] * (64 + hues[1]) + 4 * hues[0] + (hues[2] * hues[3] - hues[4])) % 360;
    }

    async function deletePicture() {
        err = '';
        errFileSize = '';

        if (!pictureId) {
            return;
        }

        let res = await fetchDelete(`/auth/v1/users/${userId}/picture/${pictureId}`);
        if (res.error) {
            err = res.error.message;
        } else {
            pictureId = undefined;
        }
    }

    async function upload(list: FileList) {
        err = '';
        errFileSize = '';

        if (!config) {
            return;
        }

        isUploading = true;

        let url = `/auth/v1/users/${userId}/picture`;

        for (let file of list) {
            if (!accept.includes(file.type)) {
                err = 'Invalid File Format, allowed: ' + accept + ', found: ' + file.type;
                break;
            }
            if (file.size > config.content_len_limit) {
                errFileSize = `${t.common.maxFileSize}: ${config.content_len_limit / 1024 / 1024} MB`;
                break;
            }

            let fd = new FormData();
            fd.append(file.name, file);
            let res = await fetch(url, {
                method: 'PUT',
                headers: {
                    'x-csrf-token': getCsrfToken(),
                },
                body: fd,
            });

            if (res.ok) {
                pictureId = await res.text();
                setTimeout(() => {
                    refDel?.focus();
                }, 100);
            } else {
                let error: ErrorResponse = await res.json();
                if (res.status === 406) {
                    err = 'File too big: ' + error.message;
                } else {
                    err = error.message || 'Upload Error';
                }
            }
        }

        files = undefined;
        isUploading = false;
    }
</script>

{#snippet avatar()}
    {#if err || !pictureId}
        <span class="absolute inset-0 flex items-center justify-center font-mono font-semibold text-white select-none {size === 'large' ? 'text-3xl' : size === 'small' ? 'text-xs' : 'text-base'}">
            {fallbackCharacters ? fallbackCharacters.toUpperCase() : ''}
        </span>
    {:else}
        <img
            src={isExternal ? pictureId : `/auth/v1/users/${userId}/picture/${pictureId}`}
            referrerpolicy="no-referrer"
            loading="lazy"
            class="absolute inset-0 h-full w-full object-cover"
            aria-label="Avatar"
            alt=""
            {onerror}
        />
    {/if}
{/snippet}

<div class="flex items-center gap-1">
    {#if config?.upload_allowed && !disableUpload}
        <form
            class="relative shrink-0 overflow-hidden rounded-full"
            aria-dropeffect="move"
            aria-label="Upload"
            style:background-color={color}
            style:width
            style:height={width}
            {onmouseenter}
            {onmouseleave}
        >
            <label
                for={id}
                aria-controls={id}
                aria-disabled={isUploading}
                class="absolute inset-0 z-10 flex cursor-pointer items-center justify-center bg-black/50 text-white transition-opacity {!isUploading && showUpload ? 'opacity-100' : 'opacity-0'}"
            >
                <IconUpload width="50%" />
            </label>
            <input
                tabindex="0"
                {id}
                type="file"
                class="sr-only"
                disabled={isUploading}
                aria-label="Upload Picture"
                aria-disabled={isUploading}
                onfocusin={() => (showUpload = true)}
                onfocusout={() => (showUpload = false)}
                {accept}
                bind:files
            />

            {#if errFileSize}
                <div class="bg-destructive text-destructive-foreground absolute inset-x-0 bottom-0 z-20 p-1 text-center text-[10px]">
                    {errFileSize}
                </div>
            {/if}

            {@render avatar()}
        </form>

        {#if size === 'large' && pictureId && !isExternal}
            <Button bind:ref={refDel} invisible level={-3} onclick={deletePicture}>
                <span title={t.common.delete}>
                    <IconStop width="1.2rem" />
                </span>
            </Button>
        {/if}
    {:else}
        <span class="relative shrink-0 overflow-hidden rounded-full" style:background-color={color} style:width style:height={width}>
            {@render avatar()}
        </span>
    {/if}
</div>
