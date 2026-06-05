<script lang="ts">
	import type { InspectItem, PlayerState } from '$lib/types';
	import { LoaderCircle } from '@lucide/svelte';
	import PlayerControls from './PlayerControls.svelte';
	import PlayerProgress from './PlayerProgress.svelte';
	import VideoMeta from './VideoMeta.svelte';

	type Props = {
		item: InspectItem | null;
		isMutationPending: boolean;
		playerState: PlayerState | null;
		onChanged: () => Promise<void> | void;
		onError: (message: string) => void;
	};

	let { item, isMutationPending, playerState, onChanged, onError }: Props = $props();
	let info = $derived(item?.track_info);
	let isGreyBorder = $derived(!info || (playerState && playerState.state === 'paused'));
</script>

<div>
	<h1 class="mb-1 text-lg font-semibold tracking-tight">Now Playing</h1>
	<div
		class="relative overflow-hidden rounded-md border-[3px] border-solid transition-transform {isGreyBorder
			? 'border-muted'
			: 'shiny-border border-transparent'} {playerState?.state === 'paused' ? 'scale-99' : 'scale-100'}"
	>
		{#if !info}
			<div class="flex aspect-video items-center justify-center bg-muted/95">
				{#if !item && isMutationPending}
					<LoaderCircle class="spin-icon size-8 text-muted-foreground" />
				{/if}
			</div>
		{/if}
		<div class="relative">
			{#if info}
				<img src={info.thumbnail} alt={`${info.title} thumbnail`} class="aspect-video bg-muted" />
			{/if}
			{#if playerState === null && item}
				<div class="absolute top-0 left-0 flex h-full w-full select-none items-center justify-center">
					<LoaderCircle class="spin-icon size-8 text-white/50" />
				</div>
			{/if}
			<PlayerProgress {playerState} {onChanged} {onError} />
		</div>
		<div class="p-4">
			{#if info}
				<h3 class="mb-1 text-center text-lg leading-6 font-medium">{info.title}</h3>
				<p class="text-center text-sm text-secondary-foreground">{info.channel}</p>
				<div class="absolute top-1 right-1 flex items-center justify-center gap-1">
					<VideoMeta
						acodec={info.acodec}
						vcodec={info.vcodec}
						track_type={info.track_type}
						width={info.width}
						height={info.height}
					/>
				</div>
			{:else if !item}
				<h3 class="text-center text-lg font-medium text-muted-foreground">
					{isMutationPending ? 'Adding to queue...' : 'Nothing playing'}
				</h3>
				<p class="text-center text-sm text-muted-foreground">
					{isMutationPending ? 'Just a sec' : 'Add something to the queue'}
				</p>
			{/if}
			<PlayerControls jobId={item?.job_id ?? null} playerState={playerState?.state ?? null} {onChanged} {onError} />
		</div>
	</div>
</div>
