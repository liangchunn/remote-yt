<script lang="ts">
	import type { HTMLImgAttributes } from 'svelte/elements';

	type Props = Omit<HTMLImgAttributes, 'src'> & {
		src: string;
		fallbackSrc?: string;
	};

	let { src, fallbackSrc, alt = '', ...rest }: Props = $props();
	let useFallback = $state(false);
	let hidden = $state(false);

	let currentSrc = $derived(useFallback && fallbackSrc ? fallbackSrc : src);

	function handleError() {
		if (fallbackSrc && !useFallback) {
			useFallback = true;
		} else {
			hidden = true;
		}
	}
</script>

{#if !hidden}
	<img src={currentSrc} {alt} onerror={handleError} {...rest} />
{/if}
