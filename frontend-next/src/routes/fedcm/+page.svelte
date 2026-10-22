<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import { onMount } from 'svelte';
    import Button from '$lib5/button/Button.svelte';
    import Input from '$lib5/form/Input.svelte';
    import CheckIcon from '$lib5/CheckIcon.svelte';
    import Main from '$lib5/Main.svelte';
    import ContentCenter from '$lib5/ContentCenter.svelte';
    import { PATTERN_URI } from '$utils/patterns';
    import Form from '$lib5/form/Form.svelte';
    import { genKey } from '$utils/helpers';

    let isSupported = $state(false);
    let isLoggedIn = $state(false);
    let credentials = $state('');
    let credentialType = $state('');

    let clientId = $state('fedcm');
    let configUrl = $state('any');

    onMount(async () => {
        configUrl = `${window.location.origin}/auth/v1/fed_cm/config`;
        clientId = `${window.location.origin}/auth/v1/fed_cm/client_config`;

        let res = await fetch('/auth/v1/fed_cm/status');
        if (res.status === 200) {
            console.log('FedCM status is: logged-in');
            isLoggedIn = true;
        } else {
            console.log('FedCM status is: logged-out');
        }

        // @ts-ignore
        if (
            window['IdentityProvider' as any] && // Is FedCM available?
            // @ts-ignore
            IdentityProvider.register != undefined // Is the IdP Registration API available?
        ) {
            console.log('FedCM is supported');
            isSupported = true;
        } else {
            console.error('FedCM is not supported');
        }
    });

    async function login() {
        // console.log('using credentials get values: configUrl: ' + configUrl + ' / clientId: ' + clientId);

        try {
            let creds = await navigator.credentials.get({
                // @ts-ignore
                identity: {
                    // context: "sign-in",
                    mode: 'button', // button mode will actually open the `login_url` if `logged-out`
                    providers: [
                        {
                            configURL: configUrl,
                            clientId: clientId,
                            // CAUTION: the genKey is not crypto safe - just for testing!
                            nonce: genKey(48),
                        },
                    ],
                },
            });

            console.log(creds);
            // @ts-ignore
            credentialType = creds.type;
            // @ts-ignore
            credentials = creds.token;
            isLoggedIn = true;
        } catch (err) {
            console.error('FedCM credentials error: ' + err);
        }
    }

    function register() {
        // @ts-ignore
        IdentityProvider.register(configUrl);
    }
</script>

<Main>
    <ContentCenter>
        <Card.Root class="w-full max-w-md">
            <Card.Header>
                <Card.Title class="text-xl">FedCM Testing</Card.Title>
            </Card.Header>
            <Card.Content class="grid gap-4">
                <div class="flex items-center gap-2">
                    <b>FedCM supported:</b>
                    <CheckIcon checked={isSupported} />
                </div>
                {#if !isSupported}
                    <p class="text-muted-foreground text-sm">
                        -> enable idp registration:<br />
                        <code class="bg-muted rounded px-1">chrome://flags/#fedcm-idp-registration</code>
                    </p>
                {/if}

                {#if isSupported}
                    <div>
                        <Button onclick={register} level={2}>Register IdP</Button>
                    </div>

                    <Form action="" onSubmit={login}>
                        <Input
                            bind:value={clientId}
                            autocomplete="off"
                            label="Client ID"
                            placeholder="Client ID"
                            required
                            pattern={PATTERN_URI}
                        />
                        <Input
                            bind:value={configUrl}
                            autocomplete="off"
                            label="Config URL"
                            placeholder="Config URL"
                            pattern={PATTERN_URI}
                        />
                        <div>
                            <Button type="submit">Login</Button>
                        </div>
                    </Form>

                    <div class="flex items-center gap-2">
                        <b>Logged In:</b>
                        <CheckIcon checked={isLoggedIn} />
                    </div>
                {/if}

                {#if credentials}
                    <div class="grid gap-2 border-t pt-3 text-sm">
                        <div class="flex gap-2">
                            <b>Credential Type:</b>
                            <span>{credentialType}</span>
                        </div>
                        <pre class="bg-muted max-h-48 overflow-auto rounded-xl p-3 text-xs break-all whitespace-pre-wrap">{credentials}</pre>
                    </div>
                {/if}
            </Card.Content>
        </Card.Root>
    </ContentCenter>
</Main>
