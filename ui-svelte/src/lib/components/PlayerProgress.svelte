<script lang="ts">
	import { executeCommand } from '$lib/api';
	import { formatTime } from '$lib/format-time';
	import type { PlayerState } from '$lib/types';
	import { buttonVariants } from '$lib/components/ui/button/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import { Slider } from '$lib/components/ui/slider/index.js';
	import { Volume1, Volume2, VolumeX } from '@lucide/svelte';

	type Props = {
		playerState: PlayerState | null;
		onChanged: () => Promise<void> | void;
		onError: (message: string) => void;
	};

	let { playerState, onChanged, onError }: Props = $props();
	let isDragging = $state(false);
	let dragTime: number | null = $state(null);
	let barRef: HTMLDivElement | undefined = $state();
	let volumeOpen = $state(false);
	let localVolume: number | null = $state(null);

	let currentTime = $derived(isDragging && dragTime !== null ? dragTime : playerState?.time || 0);
	let progressPercent = $derived((currentTime / (playerState?.length || 1)) * 100);
	let currString = $derived(formatTime(currentTime));
	let totalString = $derived(playerState ? formatTime(playerState.length) : '');
	let volumePercent = $derived(playerState ? toVolumePercent(playerState.volume) : 0);
	let displayedVolume = $derived(localVolume ?? volumePercent);
	let volumeValue = $derived(displayedVolume);

	function toVolumePercent(volume: number) {
		const clampedVolume = Math.min(Math.max(volume, 0), 255);
		return Math.round((clampedVolume / 255) * 100);
	}

	function updateTimeFromPointer(event: PointerEvent) {
		if (!barRef || !playerState) return;
		const rect = barRef.getBoundingClientRect();
		const paddingX = 16;
		const usableWidth = rect.width - paddingX * 2;
		const offsetX = Math.min(Math.max(event.clientX - rect.left - paddingX, 0), usableWidth);
		dragTime = Math.round((offsetX / usableWidth) * playerState.length);
	}

	function handlePointerDown(event: PointerEvent) {
		if (!playerState || !barRef) return;
		isDragging = true;
		barRef.setPointerCapture(event.pointerId);
		updateTimeFromPointer(event);
	}

	function handlePointerMove(event: PointerEvent) {
		if (!isDragging || !playerState) return;
		updateTimeFromPointer(event);
	}

	async function handlePointerUp(event: PointerEvent) {
		if (!playerState || !barRef) return;
		try {
			if (isDragging && dragTime !== null) {
				await seekToTime(dragTime);
			}
		} catch (error) {
			onError(error instanceof Error ? error.message : 'Seek failed');
		} finally {
			isDragging = false;
			dragTime = null;
			barRef.releasePointerCapture(event.pointerId);
		}
	}

	async function seekToTime(time: number) {
		await executeCommand({ SeekTo: time });
		await onChanged();
	}

	async function handleSeekKeydown(event: KeyboardEvent) {
		if (!playerState) return;

		const step = 10;
		let nextTime: number | null = null;
		if (event.key === 'ArrowLeft') nextTime = Math.max(playerState.time - step, 0);
		if (event.key === 'ArrowRight') nextTime = Math.min(playerState.time + step, playerState.length);
		if (event.key === 'Home') nextTime = 0;
		if (event.key === 'End') nextTime = playerState.length;

		if (nextTime === null) return;
		event.preventDefault();
		try {
			await seekToTime(nextTime);
		} catch (error) {
			onError(error instanceof Error ? error.message : 'Seek failed');
		}
	}

	async function changeVolume(value: number) {
		localVolume = value;
		try {
			await executeCommand({ SetVolume: value });
			await onChanged();
		} catch (error) {
			onError(error instanceof Error ? error.message : 'Volume update failed');
		}
	}
</script>

{#if playerState}
	<div class="transition-opacity duration-300 ease-in-out">
		<div class="absolute bottom-0 left-0 h-24 w-full bg-linear-to-t from-black/70 to-black/0"></div>
		<div class="absolute bottom-6.5 left-0 pl-4">
			<p class="font-mono text-sm tracking-tight text-white/80">
				<span>{currString}</span><span class="mx-0.5">/</span><span>{totalString}</span>
			</p>
		</div>
		<div class="absolute right-0 bottom-6.5 pr-4">
			<Popover.Root bind:open={volumeOpen}>
				<Popover.Trigger aria-label={`Volume ${displayedVolume}%`} class={`${buttonVariants({ variant: 'ghost', size: 'icon-sm' })} cursor-pointer hover:bg-muted/15 aria-expanded:bg-transparent aria-expanded:hover:bg-muted/15`}>
					{#if volumePercent === 0}
						<VolumeX class="size-4 text-white/80" />
					{:else if volumePercent <= 50}
						<Volume1 class="size-4 text-white/80" />
					{:else}
						<Volume2 class="size-4 text-white/80" />
					{/if}
				</Popover.Trigger>
				<Popover.Content side="top" align="center" sideOffset={2} class="h-36 w-10 items-center gap-1 bg-background/70 px-1 py-2 backdrop-blur-lg">
					<span class="text-xs font-medium tabular-nums">{displayedVolume}%</span>
					<Slider
						aria-label="Volume"
						type="single"
						orientation="vertical"
						min={0}
						max={100}
						value={volumeValue}
						class="h-28"
						onValueCommit={(value: number) => changeVolume(value)}
					/>
				</Popover.Content>
			</Popover.Root>
		</div>
		<div
			class="absolute bottom-1.5 left-0 h-6 w-full cursor-pointer touch-none px-4"
			bind:this={barRef}
			role="slider"
			tabindex="0"
			aria-label="Playback position"
			aria-valuemin="0"
			aria-valuemax={playerState.length}
			aria-valuenow={currentTime}
			onkeydown={handleSeekKeydown}
			onpointerdown={handlePointerDown}
			onpointermove={handlePointerMove}
			onpointerup={handlePointerUp}
		>
			<div class="relative h-full">
				<div class="absolute top-1/2 left-0 h-1 w-full -translate-y-1/2 rounded-full bg-white/50"></div>
				<div
					class="absolute top-1/2 left-0 h-1 -translate-y-1/2 rounded-full bg-red-500"
					style={`width: ${progressPercent}%`}
				></div>
				<div
					class="absolute top-1/2 z-10 h-3 w-3 -translate-y-1/2 rounded-full bg-red-500 shadow"
					style={`left: calc(${progressPercent}% - 6px); transition: ${isDragging ? 'none' : 'left 0.1s linear'}`}
				></div>
			</div>
		</div>
	</div>
{/if}
