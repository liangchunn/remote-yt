<script lang="ts">
	import type { InspectItem, PlayerState } from '$lib/types';
	import ClearAllButton from './ClearAllButton.svelte';
	import NowPlaying from './NowPlaying.svelte';
	import QueueItem from './QueueItem.svelte';

	type Props = {
		error: boolean;
		queue: InspectItem[];
		nowPlaying: InspectItem | null;
		playerState: PlayerState | null;
		isMutationPending: boolean;
		onChanged: () => Promise<void> | void;
		onError: (message: string) => void;
	};

	let {
		error,
		queue,
		nowPlaying,
		playerState,
		isMutationPending,
		onChanged,
		onError
	}: Props = $props();
</script>

{#if error}
	<p class="mt-4 text-center text-lg text-destructive">Server offline</p>
{:else}
	<div class="flex flex-col gap-4">
		<NowPlaying item={nowPlaying} {isMutationPending} {playerState} {onChanged} {onError} />
		{#if queue.length !== 0 || (queue.length === 0 && !!nowPlaying && isMutationPending)}
			<div>
				<h1 class="mb-1 text-lg font-semibold tracking-tight">Up Next</h1>
				<div class="flex flex-col gap-2">
					{#each queue as item (item.job_id)}
						<QueueItem {item} {onChanged} {onError} />
					{/each}
					{#if isMutationPending}
						<QueueItem item={null} {onChanged} {onError} />
					{/if}
				</div>
			</div>
		{/if}
		<ClearAllButton show={!!nowPlaying || queue.length > 0} {onChanged} {onError} />
	</div>
{/if}
