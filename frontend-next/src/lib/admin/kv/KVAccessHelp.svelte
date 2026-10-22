<script lang="ts">
    import Button from '$lib/button/Button.svelte';
    import Modal from '$lib/Modal.svelte';
    import { useI18nAdmin } from '$state/i18n_admin.svelte';
    import IconClipboard from '$icons/IconClipboard.svelte';

    let ta = useI18nAdmin();

    let showModal = $state(false);
    let closeModal: undefined | (() => void) = $state();

    const curl = 'curl -s \\';
    const headerAuth = "-H 'Authorization: Bearer {key_id}${key_secret}' \\";
    const headerJson = "-H 'Content-Type: application/json' \\";
    let urlBase = $derived(`${window.location.origin}/auth/v1/kv`);

    let testKey = $derived(`${curl}
    ${headerAuth}
    ${urlBase}/test`);

    let getAllKeys = $derived(`${curl}
    ${headerAuth}
    ${urlBase}/keys`);

    let getAllValues = $derived(`${curl}
    ${headerAuth}
    ${urlBase}/values`);

    let getAllValuesParams = $derived(`${curl}
    ${headerAuth}
    '${urlBase}/values?limit=10&search=myKey'`);

    const jsonDataLeft = `--data-raw '{
        "key": "myKey",
        "encrypted": false,
        "value": `;
    const jsonDataRight = `
    }' \\`;

    let setKeyValueString = $derived(`curl -s -X PUT \\
    ${headerAuth}
    ${headerJson}
    ${jsonDataLeft} "Some String 123"${jsonDataRight}
    ${urlBase}/keys`);

    let setKeyValueNumber = $derived(`curl -s -X PUT \\
    ${headerAuth}
    ${headerJson}
    ${jsonDataLeft} 1337${jsonDataRight}
    ${urlBase}/keys`);

    let setKeyValueBool = $derived(`curl -s -X PUT \\
    ${headerAuth}
    ${headerJson}
    ${jsonDataLeft} true${jsonDataRight}
    ${urlBase}/keys`);

    let setKeyValueArray = $derived(`curl -s -X PUT \\
    ${headerAuth}
    ${headerJson}
    ${jsonDataLeft} [1, 2, 3]${jsonDataRight}
    ${urlBase}/keys`);

    let setKeyValueObj = $derived(`curl -s -X PUT \\
    ${headerAuth}
    ${headerJson}
    ${jsonDataLeft} {"thisCanBe": "any valid json", "num": 1337, "andBool": true}${jsonDataRight}
    ${urlBase}/keys`);

    let getValue = $derived(`${curl}
    ${headerAuth}
    ${urlBase}/keys/myKey`);

    let deleteKey = $derived(`curl -s -X DELETE \\
    ${headerAuth}
    ${urlBase}/keys/myKey`);
</script>

<Button level={3} onclick={() => (showModal = true)}>
    {ta.kv.help.help}
</Button>

{#snippet code(value: string)}
    <div class="relative">
        <div class="absolute top-1 right-1">
            <Button invisible onclick={() => navigator.clipboard.writeText(value)} ariaLabel="copy">
                <IconClipboard width="1.1rem" />
            </Button>
        </div>
        <pre class="bg-muted/50 max-w-full overflow-x-auto rounded-xl border p-3 pr-10 text-xs"><code>{value}</code></pre>
    </div>
{/snippet}

<Modal bind:showModal bind:closeModal class="sm:max-w-3xl">
    <div class="grid max-h-[75dvh] gap-3 overflow-y-auto pr-1">
        <h1 class="text-xl font-semibold">{ta.kv.help.help}</h1>

        <p class="text-sm">{ta.kv.help.p1}</p>
        <ul class="list-disc pl-5 text-sm">
            {#each [0, 1, 2, 3, 4, 5] as i (i)}
                <li>{ta.kv.help.ops[i]}</li>
            {/each}
        </ul>
        <p class="text-muted-foreground text-sm">{@html ta.kv.help.p2}</p>
        <p class="text-muted-foreground text-sm">{@html ta.kv.help.p3}</p>

        <h2 class="font-semibold">{ta.kv.help.ops[0]}</h2>
        {@render code(testKey)}

        <h2 class="font-semibold">{ta.kv.help.ops[1]}</h2>
        {@render code(getAllKeys)}

        <h2 class="font-semibold">{ta.kv.help.ops[2]}</h2>
        {@render code(getAllValues)}
        <p class="text-sm">URL Parameters:</p>
        <ul class="list-disc pl-5 text-sm">
            <li><code>limit: u32</code></li>
            <li><code>search: String</code></li>
        </ul>
        {@render code(getAllValuesParams)}

        <h2 class="font-semibold">{ta.kv.help.ops[3]}</h2>
        {@render code(setKeyValueString)}
        {@render code(setKeyValueNumber)}
        {@render code(setKeyValueBool)}
        {@render code(setKeyValueArray)}
        {@render code(setKeyValueObj)}

        <h2 class="font-semibold">{ta.kv.help.ops[4]}</h2>
        {@render code(getValue)}

        <h2 class="font-semibold">{ta.kv.help.ops[5]}</h2>
        {@render code(deleteKey)}
    </div>
</Modal>
