import type { Command, HistoryEntry, InspectApi, JobType } from './types';

async function parseResponse<T>(response: Response): Promise<T> {
	const text = await response.text();
	const data = text ? JSON.parse(text) : null;

	if (!response.ok || data?.error) {
		throw new Error(data?.error ?? `Request failed with ${response.status}`);
	}

	return data as T;
}

export async function fetchInspect() {
	const response = await fetch('/api/inspect');
	return parseResponse<InspectApi>(response);
}

export async function fetchHistory() {
	const response = await fetch('/api/history');
	return parseResponse<HistoryEntry[]>(response);
}

export async function queueMedia(jobType: JobType, url: string, height: number) {
	const response = await fetch('/api/queue', {
		method: 'POST',
		body: JSON.stringify({
			url,
			type: jobType,
			height: jobType === 'Queue' ? undefined : height
		}),
		headers: {
			'Content-Type': 'application/json'
		}
	});

	return parseResponse<unknown>(response);
}

export async function executeCommand(command: Command) {
	await fetch('/api/execute_command', {
		method: 'POST',
		body: JSON.stringify(command),
		headers: {
			'Content-Type': 'application/json'
		}
	});
}

export async function cancelJob(jobId: string) {
	await fetch(`/api/cancel/${jobId}`, { method: 'POST' });
}

export async function swapJob(jobId: string) {
	await fetch(`/api/swap/${jobId}`, { method: 'POST' });
}

export async function moveJob(jobId: string, newPos: number) {
	await fetch(`/api/move/${jobId}/${newPos}`, { method: 'POST' });
}

export async function clearQueue() {
	await fetch('/api/clear', { method: 'POST' });
}

export async function removeHistoryEntry(webpageUrl: string) {
	await fetch('/api/remove_history', {
		method: 'POST',
		body: JSON.stringify({ webpage_url: webpageUrl }),
		headers: {
			'Content-Type': 'application/json'
		}
	});
}
