<script lang="ts">
	import type { JobType } from '$lib/types';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { ClipboardPaste, Plus } from '@lucide/svelte';

	const qualityToMinHeight = {
		sd: 480,
		hd: 720,
		fhd: 1080,
		sd_s: 480,
		hd_s: 720,
		fhd_s: 1080,
		config: 0
	};

	type Quality = keyof typeof qualityToMinHeight;

	const qualityItems: { value: Quality; label: string }[] = [
		{ value: 'config', label: 'Auto' },
		{ value: 'sd_s', label: '480p' },
		{ value: 'hd_s', label: '720p' },
		{ value: 'fhd_s', label: '1080p' },
		{ value: 'sd', label: '480m' }
	];

	type Props = {
		onQueue: (jobType: JobType, url: string, height: number) => Promise<void> | void;
	};

	let { onQueue }: Props = $props();
	let open = $state(false);
	let url = $state('');
	let quality: Quality = $state('config');

	async function pasteUrl() {
		url = await navigator.clipboard.readText();
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		const minHeight = qualityToMinHeight[quality];
		const jobType: JobType = quality === 'config' ? 'Queue' : quality.endsWith('_s') ? 'QueueSplit' : 'QueueMerged';
		const queuedUrl = url;

		url = '';
		open = false;
		await onQueue(jobType, queuedUrl, minHeight);
	}
</script>

<Button onclick={() => (open = true)}>
	<Plus /> Queue...
</Button>

<Dialog.Root bind:open>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Queue Media</Dialog.Title>
			<Dialog.Description>Add a URL to the playback queue.</Dialog.Description>
		</Dialog.Header>

		<form id="queue-form" class="grid gap-4" onsubmit={handleSubmit}>
			<div class="grid gap-2">
				<label for="url" class="text-sm font-medium">URL</label>
				<div class="flex gap-2">
					<Input id="url" bind:value={url} placeholder="Insert URL..." />
					<Button type="button" variant="outline" size="icon" aria-label="Paste URL from clipboard" onclick={pasteUrl}>
						<ClipboardPaste />
					</Button>
				</div>
			</div>

			<div class="grid gap-2">
				<label for="quality" class="text-sm font-medium">Media Type</label>
				<Select.Root type="single" bind:value={quality} items={qualityItems}>
					<Select.Trigger id="quality" class="w-full">
						{qualityItems.find((item) => item.value === quality)?.label ?? 'Auto'}
					</Select.Trigger>
					<Select.Content>
						{#each qualityItems as item (item.value)}
							<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>
		</form>

		<Dialog.Footer>
			<Button type="submit" form="queue-form">Queue</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
