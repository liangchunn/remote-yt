<script lang="ts">
	import { clearQueue } from '$lib/api';
	import { buttonVariants } from '$lib/components/ui/button/index.js';
	import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
	import { X } from '@lucide/svelte';

	type Props = {
		show: boolean;
		onChanged: () => Promise<void> | void;
		onError: (message: string) => void;
	};

	let { show, onChanged, onError }: Props = $props();
	let open = $state(false);

	async function confirm() {
		try {
			open = false;
			await clearQueue();
			await onChanged();
		} catch (error) {
			onError(error instanceof Error ? error.message : 'Clear failed');
		}
	}
</script>

{#if show}
	<div class="flex justify-center">
		<AlertDialog.Root bind:open>
			<AlertDialog.Trigger class={`${buttonVariants({ size: 'sm', variant: 'ghost' })} text-muted-foreground`}>
				<X /> Clear all
			</AlertDialog.Trigger>
			<AlertDialog.Content>
				<AlertDialog.Header>
					<AlertDialog.Title>Clear everything and stop player?</AlertDialog.Title>
					<AlertDialog.Description>
						This will clear everything in the queue and stop the player. This action cannot be undone.
					</AlertDialog.Description>
				</AlertDialog.Header>
				<AlertDialog.Footer>
					<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
					<AlertDialog.Action onclick={confirm}>Clear and stop</AlertDialog.Action>
				</AlertDialog.Footer>
			</AlertDialog.Content>
		</AlertDialog.Root>
	</div>
{/if}
