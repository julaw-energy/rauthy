<script lang="ts">
    import { toast } from 'svelte-sonner';
    import { Plus, X } from '@lucide/svelte';
    import { fetchDelete, fetchPost, fetchPut } from '$api/fetch';
    import type {
        ChannelSettingsRequest,
        ChannelSettingsResponse,
        HttpKvRequest,
        HttpKvResponse,
        SettingsTestResponse,
    } from '$api/types/settings';
    import InputCheckbox from '$lib5/form/InputCheckbox.svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import InputPassword from '$lib5/form/InputPassword.svelte';
    import InputSelect from '$lib5/form/InputSelect.svelte';
    import { Badge } from '$lib/components/ui/badge';

    // One channel: its driver, how it is called, who it delivers to and when it expires. Used for
    // the existing channels and, without `channel`, for a new one.
    let {
        channel,
        attrs,
        onChange,
        onCancel,
    }: {
        channel?: ChannelSettingsResponse;
        attrs: string[];
        onChange: () => void;
        onCancel?: () => void;
    } = $props();

    interface Kv {
        name: string;
        value: string;
        secret: boolean;
        // a secret is stored already: leave `value` empty to keep it
        stored: boolean;
    }

    const toKv = (l: HttpKvResponse[]): Kv[] =>
        l.map(e => ({ name: e.name, value: e.value ?? '', secret: e.secret, stored: e.secret }));

    function init() {
        const c = channel;
        return {
            name: '',
            label: c?.label ?? '',
            driver: c?.driver ?? 'http',
            recipient: c?.recipient ?? 'phone',
            // webhook
            url: c?.url ?? '',
            apiKey: '',
            clearKey: false,
            // smtp
            smtpUrl: c?.smtp?.url ?? '',
            smtpPort: c?.smtp?.port ? String(c.smtp.port) : '',
            smtpUser: c?.smtp?.username ?? '',
            smtpPassword: '',
            smtpClearPassword: false,
            smtpFrom: c?.smtp?.from ?? '',
            smtpTls: c?.smtp?.tls_mode ?? 'starttls',
            // http
            method: c?.http?.method ?? 'POST',
            httpUrl: c?.http?.url ?? '',
            bodyFormat: c?.http?.body_format ?? 'json',
            headers: toKv(c?.http?.headers ?? []),
            body: toKv(c?.http?.body ?? []),
            expires: c?.expires ? toLocal(c.expires) : '',
        };
    }

    // `datetime-local` works with the local time without a zone
    function toLocal(ts: number) {
        const d = new Date(ts * 1000);
        const p = (n: number) => String(n).padStart(2, '0');
        return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
    }

    let d = $state(init());
    let busy = $state('');
    let err = $state('');
    let ok = $state('');
    let confirming = $state(false);

    const isNew = $derived(!channel);
    const builtin = $derived(channel?.builtin ?? false);
    const isEmail = $derived(channel?.channel === 'email');

    const driverItems = $derived([
        ...(!isNew ? [{ value: '', label: 'Disabled' }] : []),
        ...(isEmail ? [{ value: 'smtp', label: 'Built-in mailer (the server under Email (SMTP))' }] : []),
        ...(!builtin ? [{ value: 'smtp', label: 'Email through its own SMTP server' }] : []),
        { value: 'http', label: 'HTTP: describe the call to a provider' },
        { value: 'webhook', label: 'Webhook: Rauthy\'s own signed format' },
    ]);

    const recipientItems = $derived([
        { value: 'email', label: 'Email address of the user' },
        { value: 'phone', label: 'Phone number of the user' },
        ...attrs.map(a => ({ value: `attr:${a}`, label: `Attribute: ${a}` })),
    ]);

    const tlsItems = [
        { value: 'tls', label: 'TLS (port 465)' },
        { value: 'starttls', label: 'STARTTLS (port 587)' },
        { value: 'danger-insecure', label: 'None / insecure (local test servers only)' },
    ];

    const kvReq = (l: Kv[]): HttpKvRequest[] =>
        l
            .filter(e => e.name.trim())
            .map(e => ({
                name: e.name.trim(),
                // an empty secret keeps the stored one
                value: e.secret && e.stored && !e.value ? undefined : e.value,
                secret: e.secret,
            }));

    function payload(expires?: number): ChannelSettingsRequest {
        let p: ChannelSettingsRequest = {
            driver: d.driver,
            label: d.label.trim() || undefined,
            recipient: builtin ? undefined : d.recipient,
        };
        if (!builtin) {
            if (expires !== undefined) p.expires = expires;
            else if (d.expires) p.expires = Math.floor(new Date(d.expires).getTime() / 1000);
        }
        if (d.driver === 'webhook') {
            p.url = d.url.trim() || undefined;
            if (d.clearKey) p.api_key = '';
            else if (d.apiKey) p.api_key = d.apiKey;
        } else if (d.driver === 'smtp' && !isEmail) {
            p.smtp = {
                url: d.smtpUrl.trim() || undefined,
                port: d.smtpPort ? Number(d.smtpPort) : undefined,
                username: d.smtpUser.trim() || undefined,
                password: d.smtpClearPassword ? '' : d.smtpPassword || undefined,
                from: d.smtpFrom.trim(),
                tls_mode: d.smtpTls,
            };
        } else if (d.driver === 'http') {
            p.http = {
                method: d.method,
                url: d.httpUrl.trim(),
                headers: kvReq(d.headers),
                body_format: d.bodyFormat,
                body: kvReq(d.body),
            };
        }
        return p;
    }

    async function save(expires?: number) {
        err = '';
        ok = '';
        let name = channel?.channel ?? d.name.trim();
        if (!/^[a-z][a-z0-9_]{0,31}$/.test(name)) {
            err = 'The name needs lowercase letters, digits or _, starting with a letter';
            return;
        }
        busy = 'save';
        let res = await fetchPut(`/auth/v1/settings/channel/${name}`, payload(expires));
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success(isNew ? `Channel ${name} created` : `${d.label || name} saved`);
        if (isNew) onCancel?.();
        onChange();
    }

    // after this the channel no longer sends, and can be deleted
    function expireNow() {
        save(Math.floor(Date.now() / 1000) - 1);
    }

    async function post(path: string, key: string) {
        err = '';
        ok = '';
        busy = key;
        let res = await fetchPost<SettingsTestResponse>(`/auth/v1/settings/channel/${channel!.channel}/${path}`);
        busy = '';
        if (res.error) err = res.error.message;
        else ok = res.body?.message ?? 'Done';
    }

    async function importFromFile() {
        busy = 'import';
        let res = await fetchPost(`/auth/v1/settings/import/channel.${channel!.channel}`);
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success('Imported from the config file');
        onChange();
    }

    async function remove() {
        if (!confirming) {
            confirming = true;
            return;
        }
        confirming = false;
        busy = 'delete';
        let res = await fetchDelete(`/auth/v1/settings/channel/${channel!.channel}`);
        busy = '';
        if (res.error) {
            err = res.error.message;
            return;
        }
        toast.success(`Channel ${channel!.channel} deleted`);
        onChange();
    }

    function preset(kind: string) {
        const secret = (name: string): Kv => ({ name, value: '', secret: true, stored: false });
        const kv = (name: string, value: string): Kv => ({ name, value, secret: false, stored: false });
        d.method = 'POST';
        if (kind === 'json') {
            d.bodyFormat = 'json';
            d.httpUrl = 'https://api.example.com/v1/messages';
            d.headers = [secret('Authorization')];
            d.body = [kv('to', '{{to}}'), kv('from', 'MyApp'), kv('text', '{{text}}')];
        } else if (kind === 'form') {
            d.bodyFormat = 'form';
            d.httpUrl = 'https://api.example.com/2010-04-01/Accounts/ACCOUNT_ID/Messages.json';
            d.headers = [secret('Authorization')];
            d.body = [kv('To', '{{to}}'), kv('From', '+15005550006'), kv('Body', '{{text}}')];
        }
    }

    const nowTs = () => Math.floor(Date.now() / 1000);
</script>

{#snippet kvEditor(list: Kv[], what: string)}
    <div class="grid gap-2">
        {#each list as e, i (i)}
            <div class="flex flex-wrap items-center gap-2">
                <input
                    class="border-input bg-background h-9 w-40 rounded-md border px-2 text-sm"
                    placeholder="name"
                    aria-label={`${what} name`}
                    bind:value={e.name}
                />
                <input
                    class="border-input bg-background h-9 min-w-40 flex-1 rounded-md border px-2 font-mono text-sm"
                    type={e.secret ? 'password' : 'text'}
                    placeholder={e.secret && e.stored ? 'stored, leave empty to keep' : 'value'}
                    aria-label={`${what} value`}
                    autocomplete="off"
                    bind:value={e.value}
                />
                <div class="text-muted-foreground text-xs">
                    <InputCheckbox bind:checked={e.secret} ariaLabel="Secret">secret</InputCheckbox>
                </div>
                <button
                    type="button"
                    class="hover:bg-accent rounded p-1"
                    aria-label={`Remove ${what}`}
                    onclick={() => list.splice(i, 1)}
                >
                    <X class="size-4" />
                </button>
            </div>
        {/each}
        <div>
            <button
                type="button"
                class="text-muted-foreground hover:text-foreground flex items-center gap-1 text-xs"
                onclick={() => list.push({ name: '', value: '', secret: false, stored: false })}
            >
                <Plus class="size-3.5" /> Add
            </button>
        </div>
    </div>
{/snippet}

<div class="bg-card grid gap-4 rounded-xl border p-4">
    <div class="flex flex-wrap items-center gap-3">
        <h4 class="font-semibold">{channel ? channel.label : 'New channel'}</h4>
        {#if channel}
            <span class="text-muted-foreground font-mono text-xs">{channel.channel}</span>
            {#if channel.expired}
                <Badge variant="destructive">Expired</Badge>
            {:else if channel.driver === ''}
                <Badge variant="outline">Disabled</Badge>
            {:else if channel.expires}
                <Badge variant="outline">
                    Closes {new Date(channel.expires * 1000).toLocaleString()}
                </Badge>
            {:else if channel.from_db}
                <Badge variant="secondary">Stored in the database</Badge>
            {:else}
                <Badge variant="outline">From the config file</Badge>
            {/if}
        {/if}
    </div>

    <div class="grid gap-4 sm:grid-cols-2">
        {#if isNew}
            <Input bind:value={d.name} label="Name" placeholder="whatsapp_user" autocomplete="off" />
        {/if}
        <Input
            bind:value={d.label}
            label="Name shown to users"
            placeholder={isNew ? 'WhatsApp' : ''}
            autocomplete="off"
        />
        {#if builtin}
            <div class="text-muted-foreground self-end pb-2 text-sm">
                Delivers to the user's {isEmail ? 'email address' : 'phone number'}.
            </div>
        {:else}
            <InputSelect bind:value={d.recipient} label="Delivers to" items={recipientItems} />
        {/if}
    </div>

    <InputSelect bind:value={d.driver} label="Driver" items={driverItems} />
    {#if channel && !builtin && channel.driver === ''}
        <p class="text-sm text-amber-600 dark:text-amber-400">
            This channel is disabled. It may have used the removed notifyd driver: choose a driver and
            save to bring it back.
        </p>
    {/if}

    {#if d.driver === 'smtp' && isEmail}
        <p class="text-muted-foreground text-xs">
            Messages go out through the server under Config → Email (SMTP).
        </p>
    {:else if d.driver === 'smtp'}
        <div class="grid gap-4 sm:grid-cols-2">
            <Input bind:value={d.smtpUrl} label="SMTP server" placeholder="smtp.example.com" autocomplete="off" />
            <Input bind:value={d.smtpPort} label="Port" typ="number" placeholder="587" autocomplete="off" />
            <Input bind:value={d.smtpUser} label="Username" autocomplete="off" />
            <div class="grid gap-1">
                <InputPassword
                    bind:value={d.smtpPassword}
                    label="Password"
                    ariaLabel="SMTP password"
                    placeholder={channel?.smtp?.has_password ? 'unchanged' : 'not set'}
                    autocomplete="new-password"
                    name="smtp-password"
                />
                {#if channel?.smtp?.has_password}
                    <div class="text-muted-foreground text-xs">
                        <InputCheckbox bind:checked={d.smtpClearPassword} ariaLabel="Remove the stored password">
                            Remove the stored password
                        </InputCheckbox>
                    </div>
                {/if}
            </div>
            <Input bind:value={d.smtpFrom} label="Sender" placeholder="My App <noreply@example.com>" autocomplete="off" />
            <InputSelect bind:value={d.smtpTls} label="Encryption" items={tlsItems} />
        </div>
    {:else if d.driver === 'webhook'}
        <Input bind:value={d.url} label="Webhook URL" placeholder="https://agent.example.com/hook" autocomplete="off" />
        <div class="grid gap-1">
            <InputPassword
                bind:value={d.apiKey}
                label="Signing secret"
                ariaLabel="Signing secret"
                placeholder={channel?.has_api_key ? 'unchanged' : 'not set'}
                autocomplete="new-password"
                name={`key-${channel?.channel ?? 'new'}`}
            />
            {#if channel?.has_api_key}
                <div class="text-muted-foreground text-xs">
                    <InputCheckbox bind:checked={d.clearKey} ariaLabel="Remove the stored secret">
                        Remove the stored secret
                    </InputCheckbox>
                </div>
            {/if}
            <span class="text-muted-foreground text-xs">
                Rauthy POSTs one fixed JSON format and signs the body with this secret in
                <code>X-Rauthy-Signature</code>. Your agent never needs a provider password from Rauthy.
            </span>
        </div>
    {:else if d.driver === 'http'}
        <div class="grid gap-4 rounded-lg border p-3">
            <div class="flex flex-wrap items-center gap-2 text-xs">
                <span class="text-muted-foreground">Start from</span>
                <Button level={3} onclick={() => preset('json')}>JSON API</Button>
                <Button level={3} onclick={() => preset('form')}>Form + auth header</Button>
            </div>
            <div class="grid gap-4 sm:grid-cols-[8rem_1fr]">
                <InputSelect
                    bind:value={d.method}
                    label="Method"
                    items={['POST', 'PUT', 'PATCH'].map(m => ({ value: m, label: m }))}
                />
                <Input bind:value={d.httpUrl} label="URL" placeholder="https://api.example.com/v1/send" autocomplete="off" />
            </div>
            <div class="grid gap-1">
                <span class="text-sm font-medium">Headers</span>
                {@render kvEditor(d.headers, 'header')}
            </div>
            <div class="grid gap-3">
                <div class="flex flex-wrap items-end gap-4">
                    <InputSelect
                        bind:value={d.bodyFormat}
                        label="Body"
                        items={[
                            { value: 'json', label: 'JSON' },
                            { value: 'form', label: 'Form (urlencoded)' },
                        ]}
                    />
                    <span class="text-muted-foreground max-w-sm pb-2 text-xs">
                        In JSON, a name like <code>messages.0.to</code> builds nested objects and lists.
                    </span>
                </div>
                {@render kvEditor(d.body, 'attribute')}
            </div>
            <p class="text-muted-foreground text-xs">
                Use <code>{'{{to}}'}</code> (the recipient) and <code>{'{{text}}'}</code> (the message)
                at least once. More: <code>{'{{subject}}'}</code> <code>{'{{html}}'}</code>
                <code>{'{{name}}'}</code> <code>{'{{purpose}}'}</code> <code>{'{{app_name}}'}</code>
                <code>{'{{channel}}'}</code> <code>{'{{idempotency_key}}'}</code>. Tick <b>secret</b> for
                keys and tokens: they are stored encrypted, never shown again and used exactly as typed.
                Any 2xx answer counts as delivered.
            </p>
        </div>
    {/if}

    {#if !builtin}
        <div class="grid gap-2 rounded-lg border p-3">
            <span class="text-sm font-medium">Expiry</span>
            <span class="text-muted-foreground text-xs">
                After this time the channel stops working, and only then can it be deleted. Tell your
                users first, with a message through this channel. Leave empty for no expiry.
            </span>
            <div class="flex flex-wrap items-center gap-2">
                <input
                    type="datetime-local"
                    class="border-input bg-background h-9 rounded-md border px-2 text-sm"
                    aria-label="Expiry"
                    bind:value={d.expires}
                />
                {#if d.expires}
                    <Button level={3} onclick={() => (d.expires = '')}>No expiry</Button>
                {/if}
                {#if channel && !channel.expired}
                    <Button level={3} onclick={expireNow} isDisabled={busy !== ''}>Expire now</Button>
                {/if}
            </div>
        </div>
    {/if}

    <div class="flex flex-wrap items-center gap-2">
        <Button onclick={() => save()} isLoading={busy === 'save'} isDisabled={busy !== ''}>
            {isNew ? 'Create' : 'Save'}
        </Button>
        {#if channel}
            <Button
                level={2}
                onclick={() => post('test', 'test')}
                isDisabled={busy !== '' || channel.driver === '' || (channel.driver === 'smtp' && isEmail) || channel.driver === 'http'}
            >
                Check connection
            </Button>
            <Button
                level={2}
                onclick={() => post('send_test', 'send')}
                isDisabled={busy !== '' || channel.driver === '' || channel.expired}
            >
                Send a test to me
            </Button>
            {#if builtin}
                <Button level={3} onclick={importFromFile} isDisabled={busy !== ''}>Import from config file</Button>
            {:else}
                <Button level={3} onclick={remove} isDisabled={busy !== '' || !channel.expired}>
                    {confirming ? 'Click again to delete' : 'Delete'}
                </Button>
            {/if}
        {:else}
            <Button level={3} onclick={() => onCancel?.()}>Cancel</Button>
        {/if}
    </div>
    {#if channel && !builtin && !channel.expired}
        <p class="text-muted-foreground text-xs">
            Delete is available once the channel has expired. Use <b>Expire now</b> to close it right away.
        </p>
    {/if}
    {#if err}<p class="text-destructive text-sm">{err}</p>{/if}
    {#if ok}<p class="text-sm">{ok}</p>{/if}
</div>
