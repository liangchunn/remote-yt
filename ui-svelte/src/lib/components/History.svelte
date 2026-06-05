<script lang="ts">
	import { formatTime, getRelativeTimeString } from '$lib/format-time';
	import type { HistoryEntry, JobType } from '$lib/types';
	import { buttonVariants } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { ChevronDown, ListEnd, Trash } from '@lucide/svelte';
	import SafeImage from './SafeImage.svelte';

	type Props = {
		open: boolean;
		entries: HistoryEntry[] | null;
		onToggle: () => void;
		onQueue: (jobType: JobType, url: string, height: number) => Promise<void> | void;
		onRemove: (url: string) => Promise<void> | void;
		onError: (message: string) => void;
	};

	let { open, entries, onToggle, onQueue, onRemove, onError }: Props = $props();

	async function run(action: () => Promise<void> | void) {
		try {
			await action();
		} catch (error) {
			onError(error instanceof Error ? error.message : 'History action failed');
		}
	}
</script>

<div>
	<button
		type="button"
		class="group flex w-full cursor-pointer items-center justify-between text-left select-none"
		onclick={onToggle}
	>
		<h1 class="mb-1 text-lg font-semibold tracking-tight group-hover:underline">History</h1>
		<ChevronDown class="size-4 transition {open ? 'rotate-180' : ''}" />
	</button>
	{#if open}
		<div class="flex flex-col gap-2">
			{#if entries}
				{#each entries as entry (`${entry.webpage_url}-${entry.inserted_at}`)}
					<div class="flex select-none items-center gap-2 overflow-hidden rounded-md border bg-white">
						<div class="relative flex min-h-20 w-36 self-stretch bg-muted">
							<SafeImage src={entry.thumbnail} class="h-full bg-muted object-cover" />
							<p class="absolute right-1 bottom-1 rounded-sm border border-black/20 bg-black/50 px-0.5 text-xs text-white/80">
								{formatTime(entry.duration)}
							</p>
						</div>
						<div class="flex-1 py-3 pl-1">
							<p class="mb-0.5 line-clamp-2 leading-5">{entry.title}</p>
							<p class="mb-0.5 text-sm text-muted-foreground">{entry.channel}</p>
							<p class="text-xs text-muted-foreground">Played {getRelativeTimeString(entry.inserted_at)}</p>
						</div>
						<div class="self-start">
							<DropdownMenu.Root>
								<DropdownMenu.Trigger class={`${buttonVariants({ variant: 'ghost', size: 'icon' })} size-8`} aria-label="Open history item menu">
									<ChevronDown />
								</DropdownMenu.Trigger>
								<DropdownMenu.Content align="end" class="w-max min-w-max">
									<DropdownMenu.Item class="whitespace-nowrap" onclick={() => run(() => onQueue(entry.job_type, entry.webpage_url, entry.height ?? 720))}>
										<ListEnd class="mr-1 size-4" /> Add to queue
									</DropdownMenu.Item>
									<DropdownMenu.Item variant="destructive" class="whitespace-nowrap" onclick={() => run(() => onRemove(entry.webpage_url))}>
										<Trash class="mr-1 size-4" /> Remove entry
									</DropdownMenu.Item>
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						</div>
					</div>
				{/each}
			{:else}
				<p class="rounded-md border bg-white p-4 text-center text-sm text-muted-foreground">Loading history...</p>
			{/if}
		</div>
	{/if}
</div>
