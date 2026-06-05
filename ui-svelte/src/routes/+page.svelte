<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchHistory, fetchInspect, queueMedia, removeHistoryEntry } from '$lib/api';
	import History from '$lib/components/History.svelte';
	import Queue from '$lib/components/Queue.svelte';
	import QueueForm from '$lib/components/QueueForm.svelte';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import type { HistoryEntry, InspectApi, JobType } from '$lib/types';
	import { toast } from 'svelte-sonner';

	let inspect = $state<InspectApi | null>(null);
	let history = $state<HistoryEntry[] | null>(null);
	let inspectError = $state(false);
	let queuePending = $state(false);
	let historyOpen = $state(false);
	let mounted = $state(false);

	let nowPlaying = $derived(inspect?.now_playing ?? null);
	let queue = $derived(inspect?.queue ?? []);
	let playerState = $derived(inspect?.player ?? null);

	onMount(() => {
		historyOpen = localStorage.getItem('historyOpen') === 'true';
		mounted = true;
		void loadInspect();
		if (historyOpen) void loadHistory();

		const interval = setInterval(loadInspect, 1000);
		return () => clearInterval(interval);
	});

	async function loadInspect() {
		try {
			const previousUrl = inspect?.now_playing?.track_info.webpage_url ?? null;
			const nextInspect = await fetchInspect();
			const nextUrl = nextInspect.now_playing?.track_info.webpage_url ?? null;

			inspect = nextInspect;
			inspectError = false;
			if (mounted && historyOpen && previousUrl !== nextUrl) {
				await loadHistory();
			}
		} catch {
			inspectError = true;
		}
	}

	async function loadHistory() {
		history = await fetchHistory();
	}

	function showToast(message: string) {
		toast.error(message);
	}

	async function queueUrl(jobType: JobType, url: string, height: number) {
		queuePending = true;
		try {
			await queueMedia(jobType, url, height);
			await loadInspect();
		} catch (error) {
			showToast(`Failed to queue: ${error instanceof Error ? error.message : 'Unknown error'}`);
		} finally {
			queuePending = false;
		}
	}

	async function removeHistory(url: string) {
		await removeHistoryEntry(url);
		await loadHistory();
	}

	function toggleHistory() {
		historyOpen = !historyOpen;
		localStorage.setItem('historyOpen', String(historyOpen));
		if (historyOpen) void loadHistory();
	}
</script>

<div class="m-auto mb-24 flex max-w-lg flex-col gap-4 p-4 pt-4">
	<div class="flex justify-end">
		<QueueForm onQueue={queueUrl} />
	</div>
	<Queue
		error={inspectError}
		{queue}
		{nowPlaying}
		{playerState}
		isMutationPending={queuePending}
		onChanged={loadInspect}
		onError={showToast}
	/>
	<History
		open={historyOpen}
		entries={history}
		onToggle={toggleHistory}
		onQueue={queueUrl}
		onRemove={removeHistory}
		onError={showToast}
	/>
</div>

<Toaster />
