<script lang="ts">
    import Cmp from '@lucide/svelte/icons/trash-2';
    import type { IconProps } from './props.ts';

    let { opacity = 0.9, width = '1.5rem', color = 'currentColor' }: IconProps = $props();
</script>

<Cmp size={width} {color} style="opacity: {opacity}" />
