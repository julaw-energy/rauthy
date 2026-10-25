<script lang="ts">
    import { onMount } from 'svelte';
    import { toast } from 'svelte-sonner';
    import { fetchDelete, fetchGet, fetchPost, fetchPut } from '$api/fetch';
    import type {
        TemplateDetail,
        TemplateInfo,
        TemplatePreviewResponse,
        TemplateTestResponse,
    } from '$api/types/message_templates';
    import type { ChannelInfo } from '$api/types/email';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputArea from '$lib5/form/InputArea.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import { Badge } from '$lib/components/ui/badge';

    const langNames: Record<string, string> = {
        default: 'Default (all languages)',
        en: 'English',
        de: 'Deutsch',
        es: 'Español',
        fr: 'Français',
        ja: '日本語',
        ko: '한국어',
        nb: 'Norsk bokmål',
        nl: 'Nederlands',
        pt: 'Português',
        ru: 'Русский',
        uk: 'Українська',
        zhhans: '简体中文',
    };

    let list: TemplateInfo[] = $state([]);
    let channels: ChannelInfo[] = $state([]);
    let id = $state('');
    let lang = $state('');
    let detail: TemplateDetail | undefined = $state();
    let values: Record<string, string> = $state({});
    let channel = $state('email');

    let preview: TemplatePreviewResponse | undefined = $state();
    let err = $state('');
    let ok = $state('');
    let warnings: string[] = $state([]);
    let busy = $state('');

    const info = $derived(list.find(t => t.id === id));
    const dirty = $derived(
        !!detail && Object.keys(values).some(k => (values[k] ?? '') !== (detail!.values[k] ?? '')),
    );
    const differsFromBuiltin = $derived(
        !!detail && Object.keys(values).some(k => (values[k] ?? '') !== (detail!.defaults[k] ?? '')),
    );

    onMount(async () => {
        let [t, c] = await Promise.all([
            fetchGet<TemplateInfo[]>('/auth/v1/settings/templates'),
            fetchGet<ChannelInfo[]>('/auth/v1/email/channels'),
        ]);
        if (c.body) channels = c.body;
        if (t.body) {
            list = t.body;
            if (list.length) await select(list[0].id);
        }
    });

    async function select(newId: string) {
        id = newId;
        let t = list.find(x => x.id === newId);
        // `default` for messages, English for emails
        await selectLang(t?.languages.includes('default') ? 'default' : 'en');
    }

    async function selectLang(l: string) {
        lang = l;
        reset();
        let res = await fetchGet<TemplateDetail>(
            `/auth/v1/settings/templates/${id}?lang=${encodeURIComponent(l)}`,
        );
        if (res.body) {
            detail = res.body;
            // every text of the template has a value, even an unset optional one
            values = Object.fromEntries(
                (list.find(t => t.id === res.body!.id)?.fields ?? []).map(f => [
                    f.name,
                    res.body!.values[f.name] ?? '',
                ]),
            );
        }
    }

    function reset() {
        preview = undefined;
        err = '';
        ok = '';
        warnings = [];
    }

    async function refreshList() {
        let t = await fetchGet<TemplateInfo[]>('/auth/v1/settings/templates');
        if (t.body) list = t.body;
    }

    async function doPreview() {
        reset();
        busy = 'preview';
        let res = await fetchPost<TemplatePreviewResponse>(`/auth/v1/settings/templates/${id}/preview`, {
            lang,
            values,
        });
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        preview = res.body;
        warnings = res.body?.warnings ?? [];
    }

    // goes to the logged in admin: the email address, or for messages the value the channel is tied to
    async function doTest() {
        reset();
        busy = 'test';
        let res = await fetchPost<TemplateTestResponse>(`/auth/v1/settings/templates/${id}/test`, {
            lang,
            values,
            channel: info?.kind === 'message' ? channel : undefined,
        });
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        ok = res.body?.message ?? 'Sent';
        warnings = res.body?.warnings ?? [];
    }

    async function save() {
        reset();
        busy = 'save';
        let res = await fetchPut<TemplateTestResponse>(`/auth/v1/settings/templates/${id}`, {
            lang,
            values,
        });
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Template saved. New messages use it right away.');
        warnings = res.body?.warnings ?? [];
        await Promise.all([selectLang(lang), refreshList()]);
        warnings = res.body?.warnings ?? [];
    }

    async function resetToBuiltin() {
        reset();
        busy = 'reset';
        let res = await fetchDelete(`/auth/v1/settings/templates/${id}?lang=${encodeURIComponent(lang)}`);
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Back to the built-in text');
        await Promise.all([selectLang(lang), refreshList()]);
    }

    const channelItems = $derived(
        channels.map(c => ({ value: c.channel, label: `${c.label} (delivers to ${recipientName(c.recipient)})` })),
    );

    function recipientName(r: string) {
        return r === 'email'
            ? 'email address'
            : r === 'phone'
              ? 'phone number'
              : r.replace('attr:', '');
    }
</script>

<div class="grid max-w-3xl gap-6">
    <div class="grid gap-2">
        <h3 class="text-xl font-semibold">Message templates</h3>
        <p class="text-muted-foreground text-sm">
            The texts of the messages Rauthy sends. The look of an email stays as it is built in, the
            wording is yours to change. A template is checked before it is saved: one with an error is
            refused and the one in use keeps working. Once saved, the next message uses it, no restart
            needed. Try a draft with <b>Preview</b> or send it to yourself with <b>Send a test to me</b>
            before saving.
        </p>
    </div>

    <div class="grid gap-4 sm:grid-cols-2">
        <InputSelect
            bind:value={() => id, v => select(v)}
            label="Message"
            items={list.map(t => ({ value: t.id, label: t.title }))}
        />
        {#if info}
            <InputSelect
                bind:value={() => lang, v => selectLang(v)}
                label="Language"
                items={info.languages.map(l => ({
                    value: l,
                    label: `${langNames[l] ?? l}${info.customized.includes(l) ? ' •' : ''}`,
                }))}
            />
        {/if}
    </div>

    <!-- `id` changes before the texts of the new template are loaded: wait for them, a field
         without a value would bind `undefined` -->
    {#if info && detail && detail.id === id}
        <div class="bg-card grid gap-4 rounded-xl border p-4">
            <div class="flex flex-wrap items-center gap-2">
                <h4 class="font-semibold">{info.title}</h4>
                {#if detail.customized}
                    <Badge variant="secondary">Changed</Badge>
                {:else}
                    <Badge variant="outline">Built-in text</Badge>
                {/if}
                {#if dirty}<Badge variant="outline" class="text-amber-600">Unsaved</Badge>{/if}
            </div>
            <p class="text-muted-foreground text-sm">{info.description}</p>

            {#each info.fields as f (f.name)}
                {#if f.multiline}
                    <InputArea
                        bind:value={values[f.name]}
                        label={f.label + (f.required ? ' *' : '')}
                        rows={f.name === 'body' ? 4 : 3}
                    />
                {:else}
                    <Input
                        bind:value={values[f.name]}
                        label={f.label + (f.required ? ' *' : '')}
                        autocomplete="off"
                    />
                {/if}
            {/each}

            {#if info.placeholders.length}
                <div class="text-muted-foreground grid gap-1 text-xs">
                    <span>You can use these in the texts:</span>
                    <ul class="grid gap-0.5">
                        {#each info.placeholders as p (p.name)}
                            <li>
                                <code class="bg-muted rounded px-1">{`{{${p.name}}}`}</code>
                                {p.description}{p.required ? ' (required)' : ''}
                            </li>
                        {/each}
                    </ul>
                </div>
            {/if}

            {#if info.kind === 'message'}
                <InputSelect
                    bind:value={channel}
                    label="Send the test through"
                    items={channelItems}
                />
                <p class="text-muted-foreground text-xs">
                    The test goes to you, to the value the channel is tied to. It fails if you have none, a
                    phone number for example.
                </p>
            {:else}
                <p class="text-muted-foreground text-xs">The test email goes to your own address.</p>
            {/if}

            <div class="flex flex-wrap items-center gap-2">
                <Button onclick={save} isLoading={busy === 'save'} isDisabled={busy !== '' || !dirty}>
                    Save
                </Button>
                <Button level={2} onclick={doPreview} isLoading={busy === 'preview'} isDisabled={busy !== ''}>
                    Preview
                </Button>
                <Button level={2} onclick={doTest} isLoading={busy === 'test'} isDisabled={busy !== ''}>
                    Send a test to me
                </Button>
                <Button
                    level={3}
                    onclick={resetToBuiltin}
                    isLoading={busy === 'reset'}
                    isDisabled={busy !== '' || !detail.customized}
                >
                    Back to built-in
                </Button>
            </div>

            {#if err}<p class="text-destructive text-sm">{err}</p>{/if}
            {#if ok}<p class="text-sm">{ok}</p>{/if}
            {#each warnings as w}
                <p class="text-sm text-amber-600 dark:text-amber-400">{w}</p>
            {/each}
            {#if !differsFromBuiltin && !detail.customized}
                <p class="text-muted-foreground text-xs">These are the built-in texts.</p>
            {/if}
        </div>

        {#if preview}
            <div class="bg-card grid gap-3 rounded-xl border p-4">
                <h4 class="font-semibold">Preview</h4>
                <div class="grid gap-1 text-sm">
                    <span class="text-muted-foreground text-xs">Subject</span>
                    <span>{preview.subject || '(none)'}</span>
                </div>
                {#if preview.html}
                    <iframe
                        title="Email preview"
                        sandbox=""
                        srcdoc={preview.html}
                        class="h-96 w-full rounded-lg border bg-white"
                    ></iframe>
                    <details class="text-sm">
                        <summary class="text-muted-foreground cursor-pointer text-xs">Plain text version</summary>
                        <pre class="bg-muted mt-2 overflow-auto rounded-lg p-3 text-xs whitespace-pre-wrap">{preview.text}</pre>
                    </details>
                {:else}
                    <pre class="bg-muted overflow-auto rounded-lg p-3 text-sm whitespace-pre-wrap">{preview.text}</pre>
                {/if}
            </div>
        {/if}
    {/if}
</div>
