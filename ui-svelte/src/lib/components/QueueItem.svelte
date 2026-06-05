<script lang="ts">
	import { cancelJob, swapJob } from '$lib/api';
	import { formatTime } from '$lib/format-time';
	import type { InspectItem } from '$lib/types';
	import { buttonVariants } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { ChevronDown, LoaderCircle, Play, Trash } from '@lucide/svelte';
	import SafeImage from './SafeImage.svelte';
	import VideoMeta from './VideoMeta.svelte';

	type Props = {
		item: InspectItem | null;
		onChanged: () => Promise<void> | void;
		onError: (message: string) => void;
	};

	let { item, onChanged, onError }: Props = $props();
	let info = $derived(item?.track_info);

	async function run(action: () => Promise<void>) {
		try {
			await action();
			await onChanged();
		} catch (error) {
			onError(error instanceof Error ? error.message : 'Queue action failed');
		}
	}
</script>

<div class="flex select-none items-center gap-2 overflow-hidden rounded-md border bg-white">
	<div class="relative flex min-h-20 w-36 self-stretch">
		{#if info}
			<SafeImage src={info.thumbnail} class="h-full bg-muted object-cover" />
		{:else}
			<div class="w-36 bg-muted/95 object-cover">
				<div class="flex aspect-video items-center justify-center">
					<LoaderCircle class="spin-icon size-6 text-muted-foreground" />
				</div>
			</div>
		{/if}
		{#if info}
			<p class="absolute right-1 bottom-1 rounded-sm border border-black/20 bg-black/50 px-0.5 text-xs text-white/80">
				{formatTime(info.duration)}
			</p>
		{/if}
	</div>
	<div class="flex-1 py-3 pl-1">
		{#if info}
			<p class="mb-0.5 leading-4">{info.title}</p>
			<p class="mb-1 text-sm text-muted-foreground">{info.channel}</p>
			<div class="flex items-center gap-1">
				<VideoMeta
					acodec={info.acodec}
					vcodec={info.vcodec}
					track_type={info.track_type}
					width={info.width}
					height={info.height}
				/>
			</div>
		{:else}
			<p class="text-muted-foreground">Adding to queue...</p>
		{/if}
	</div>
	<div class="self-start">
		<DropdownMenu.Root>
			<DropdownMenu.Trigger disabled={!item} class={`${buttonVariants({ variant: 'ghost', size: 'icon' })} size-8`} aria-label="Open queue item menu">
				<ChevronDown />
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="end">
				<DropdownMenu.Item disabled={!item} onclick={() => item && run(() => swapJob(item.job_id))}>
					<Play class="mr-1 size-4" /> Play now
				</DropdownMenu.Item>
				<DropdownMenu.Item variant="destructive" disabled={!item} onclick={() => item && run(() => cancelJob(item.job_id))}>
					<Trash class="mr-1 size-4" /> Remove item
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>
</div>
