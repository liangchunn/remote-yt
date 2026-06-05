<script lang="ts">
	import { cancelJob, executeCommand } from '$lib/api';
	import { Button } from '$lib/components/ui/button/index.js';
	import { FastForward, Pause, Play, Rewind, SkipBack, SkipForward } from '@lucide/svelte';

	type Props = {
		jobId: string | null;
		playerState: 'playing' | 'paused' | null;
		onChanged: () => Promise<void> | void;
		onError: (message: string) => void;
	};

	let { jobId, playerState, onChanged, onError }: Props = $props();

	async function run(action: () => Promise<void>) {
		try {
			await action();
			await onChanged();
		} catch (error) {
			onError(error instanceof Error ? error.message : 'Command failed');
		}
	}
</script>

<div class="mt-1 flex items-center justify-center">
	<Button variant="ghost" size="icon" class="size-12" disabled>
		<SkipBack class="size-5" />
	</Button>
	<Button
		variant="ghost"
		size="icon"
		class="size-12 cursor-pointer"
		disabled={!jobId || !playerState}
		onclick={() => run(() => executeCommand('SeekRewind'))}
	>
		<Rewind class="size-5" />
	</Button>
	<Button
		variant="ghost"
		size="icon"
		class="size-12 cursor-pointer"
		disabled={!jobId || !playerState}
		onclick={() => run(() => executeCommand('TogglePause'))}
	>
		{#if jobId && playerState === 'paused'}
			<Play class="size-5" />
		{:else}
			<Pause class="size-5" />
		{/if}
	</Button>
	<Button
		variant="ghost"
		size="icon"
		class="size-12 cursor-pointer"
		disabled={!jobId || !playerState}
		onclick={() => run(() => executeCommand('SeekForward'))}
	>
		<FastForward class="size-5" />
	</Button>
	<Button
		variant="ghost"
		size="icon"
		class="size-12 cursor-pointer"
		disabled={!jobId}
		onclick={() => jobId && run(() => cancelJob(jobId))}
	>
		<SkipForward class="size-5" />
	</Button>
</div>
