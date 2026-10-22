<script lang="ts">
    import { useI18nAdmin } from '$state/i18n_admin.svelte';

    let ta = useI18nAdmin();
</script>

<div class="bg-muted/40 grid gap-2 rounded-xl border p-4 text-sm">
    <h4 class="font-semibold">ID token claim mappings</h4>
    <p>{ta.providers.config.jsonPath.p1}</p>
    <p class="text-muted-foreground">{@html ta.providers.config.jsonPath.p2}</p>
    <p class="text-muted-foreground">{@html ta.providers.config.jsonPath.p3}</p>
    <p class="text-muted-foreground">{@html ta.providers.config.jsonPath.p4}</p>
    <p class="text-muted-foreground">{@html ta.providers.config.jsonPath.p5}</p>
    <p class="text-muted-foreground">{@html ta.providers.config.jsonPath.p6}</p>
</div>
