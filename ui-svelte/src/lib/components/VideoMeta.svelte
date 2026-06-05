<script lang="ts">
	import type { TrackInfo } from '$lib/types';

	let { acodec, vcodec, track_type, width, height }: Pick<
		TrackInfo,
		'acodec' | 'vcodec' | 'track_type' | 'width' | 'height'
	> = $props();

	let dimensions = $derived(width && height ? `${width}x${height}` : null);
	let acodecTrimmed = $derived(trimFormat(acodec));
	let vcodecTrimmed = $derived(trimFormat(vcodec));
	let combined = $derived([acodecTrimmed, vcodecTrimmed].filter(Boolean).join('+'));

	function trimFormat(codec: string): string | null {
		if (codec.length === 0) return null;
		return codec.includes('.') ? codec.split('.')[0] : codec;
	}
</script>

{#snippet badge(label: string | null)}
	{#if label}
		<div class="inline-block rounded-sm border bg-background/75 px-1 py-0.5 font-mono text-xs text-secondary-foreground">
			{label}
		</div>
	{/if}
{/snippet}

{@render badge(dimensions)}
{#if track_type === 'merged'}
	{@render badge(combined)}
{:else}
	{@render badge(vcodecTrimmed)}
	{@render badge(acodecTrimmed)}
{/if}
