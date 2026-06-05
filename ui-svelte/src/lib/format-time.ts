const relativeTimeFormatter = new Intl.RelativeTimeFormat('en-US', {
	numeric: 'auto'
});

export function formatTime(seconds: number) {
	if (seconds === 0) {
		return '--:--';
	}

	const hrs = Math.floor(seconds / 3600);
	const mins = Math.floor((seconds % 3600) / 60);
	const secs = seconds % 60;

	if (hrs > 0) {
		return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
	}

	return `${mins}:${secs.toString().padStart(2, '0')}`;
}

export function getRelativeTimeString(timestamp: number): string {
	const timeMs = timestamp * 1000;
	const deltaSeconds = Math.round((timeMs - Date.now()) / 1000);
	const cutoffs = [60, 3600, 86400, 86400 * 7, 86400 * 30, 86400 * 365, Infinity];
	const units: Intl.RelativeTimeFormatUnit[] = [
		'second',
		'minute',
		'hour',
		'day',
		'week',
		'month',
		'year'
	];
	const unitIndex = cutoffs.findIndex((cutoff) => cutoff > Math.abs(deltaSeconds));
	const divisor = unitIndex ? cutoffs[unitIndex - 1] : 1;

	return relativeTimeFormatter.format(Math.floor(deltaSeconds / divisor), units[unitIndex]);
}
