<script lang="ts">
    import { Label } from '$lib/components/ui/label';

    let {
        name,
        value = $bindable(),
        label = '',
        disabled = false,
        min,
        max,
        step = 1,
        widthRange = '15rem',
        bgMode,
        hue,
        sat,
        lum,
    }: {
        name?: string;
        value: number;
        label: string;
        disabled?: boolean | null | undefined;
        min: number;
        max: number;
        step?: number;
        widthRange?: string;
        bgMode?: 'generic' | 'hue' | 'sat' | 'lum';
        hue?: number;
        sat?: number;
        lum?: number;
    } = $props();

    const id = `range-${Math.random().toString(36).slice(2, 8)}`;

    let bg = $derived.by(() => {
        const h = hue ?? 0;
        const s = sat ?? 100;
        const l = lum ?? 50;
        switch (bgMode) {
            case 'hue':
                return 'linear-gradient(to right,hsl(0 80% 50%),hsl(60 80% 50%),hsl(120 80% 50%),hsl(180 80% 50%),hsl(240 80% 50%),hsl(300 80% 50%),hsl(360 80% 50%))';
            case 'sat':
                return `linear-gradient(to right,hsl(${h} 0% ${l}%),hsl(${h} 100% ${l}%))`;
            case 'lum':
                return `linear-gradient(to right,hsl(${h} ${s}% 0%),hsl(${h} ${s}% 50%),hsl(${h} ${s}% 100%))`;
            default:
                return undefined;
        }
    });
</script>

<div class="grid gap-1.5" style:max-width={widthRange}>
    <div class="flex items-center justify-between">
        <Label for={id}>{label}</Label>
        <span class="text-muted-foreground text-xs tabular-nums">{value}</span>
    </div>
    <input
        {id}
        {name}
        type="range"
        {min}
        {max}
        {step}
        {disabled}
        bind:value
        class="accent-primary h-2 w-full cursor-pointer appearance-none rounded-full bg-muted disabled:opacity-50 [&::-webkit-slider-thumb]:size-4 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:border [&::-webkit-slider-thumb]:bg-background"
        style:background={bg}
    />
</div>
