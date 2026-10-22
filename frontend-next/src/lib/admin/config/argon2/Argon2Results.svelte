<script lang="ts">
    import * as Table from '$lib/components/ui/table';
    import type { PasswordHashTime } from '$api/types/password_hashing.ts';

    let {
        res,
    }: {
        res: PasswordHashTime[];
    } = $props();
</script>

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head>ms</Table.Head>
            <Table.Head>t_cost</Table.Head>
            <Table.Head>m_cost</Table.Head>
            <Table.Head>p_cost</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#each res as r, i (i)}
            <Table.Row>
                <Table.Cell class="font-mono">{r.time_taken}</Table.Cell>
                <Table.Cell class="font-mono">{r.t_cost}</Table.Cell>
                <Table.Cell class="font-mono">{r.m_cost}</Table.Cell>
                <Table.Cell class="font-mono">{r.p_cost}</Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
</Table.Root>
