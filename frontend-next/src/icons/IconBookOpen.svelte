<script lang="ts">
    import Cmp from '@lucide/svelte/icons/book-open';
    import type { IconProps } from './props.ts';

    let { opacity = 0.9, width = '1.5rem', color = 'currentColor' }: IconProps = $props();
</script>

<Cmp size={width} {color} style="opacity: {opacity}" />
