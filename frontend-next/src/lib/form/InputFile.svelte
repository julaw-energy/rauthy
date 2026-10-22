<script lang="ts">
    import { genKey } from '$utils/helpers';
    import Loading from '$lib5/Loading.svelte';
    import IconUpload from '$icons/IconUpload.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import { uploadFile } from '$api/fetch';

    let {
        method = 'PUT',
        accept = ['.image/jpg', 'image/jpeg', 'image/png', 'image/svg+xml'],
        url,
        disabled,
        buttonOnly = false,
        buttonSize = '1.5rem',
        fileName,
        multiple = false,
        onSuccess,
        width = '17rem',
        height = '2.5rem',
    }: {
        method?: 'POST' | 'PUT';
        accept?: string[];
        url: string;
        disabled?: boolean;
        buttonOnly?: boolean;
        buttonSize?: string;
        fileName?: string;
        multiple?: boolean;
        onSuccess?: () => void;
        width?: string;
        height?: string;
    } = $props();

    const id = genKey();
    let ta = useI18nAdmin();

    let isUploading = $state(false);
    let dragging = $state(false);
    let files: undefined | FileList = $state();
    let filesToUpload = $state(0);
    let filesFinished = $state(0);

    $effect(() => {
        if (!isUploading && files && files.length > 0) {
            upload(files);
        }
    });

    function ondrop(ev: DragEvent) {
        ev.preventDefault();
        dragging = false;
        files = ev.dataTransfer?.files;
    }

    async function upload(list: FileList) {
        filesToUpload = list.length;
        filesFinished = 0;
        isUploading = true;

        for (let file of list) {
            if (accept && !accept.includes(file.type)) {
                filesFinished += 1;
                continue;
            }

            let res = await uploadFile(method, url, file, fileName || file.name);
            if (res.error) {
                console.error(res.error);
            } else {
                onSuccess?.();
            }

            filesFinished += 1;
        }

        files = undefined;
        isUploading = false;
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    aria-label="Upload"
    class="border-input flex items-center justify-center rounded-xl border border-dashed transition-colors {dragging ? 'bg-accent' : ''} {buttonOnly ? 'border-0' : ''}"
    style:width={buttonOnly ? undefined : width}
    style:min-height={buttonOnly ? undefined : height}
    {ondrop}
    ondragover={(ev) => {
        ev.preventDefault();
        dragging = true;
    }}
    ondragleave={() => (dragging = false)}
>
    {#if isUploading}
        <div class="flex items-center gap-2 p-2 text-sm">
            <span>{ta.common.loading}: {filesFinished} / {filesToUpload}</span>
            <Loading />
        </div>
    {:else}
        <label for={id} class="flex cursor-pointer items-center gap-2 p-2 {disabled ? 'pointer-events-none opacity-50' : ''}">
            <IconUpload width={buttonSize} />
        </label>
        <input
            type="file"
            {id}
            class="sr-only"
            aria-hidden={buttonOnly}
            accept={accept && accept.join(', ')}
            bind:files
            {disabled}
            {multiple}
        />
    {/if}
</div>
