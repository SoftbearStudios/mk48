<script>
	let body = JSON.stringify({request: 'RequestMetrics'});
</script>

<main>
	<h1>Core Metrics</h1>
	{#await fetch("/metrics/", {method: 'POST', body, headers: {'Content-Type': 'application/json'}}).then(res => res.json())}
	{:then data}
		<h2>Raw JSON</h2>
		<p>{JSON.stringify(data)}</p>
		<h2>Table</h2>

		{#each Object.entries(data.MetricsRequested.metrics) as [gameId, metrics]}
			<h3>{gameId}</h3>
			<table>
				{#each Object.entries(metrics) as [key, value]}
					<tr>
						<td>{key}</td>
						<td>{value}</td>
					</tr>
				{/each}
			</table>
		{/each}
	{:catch err}
		<p>{err}</p>
	{/await}
</main>

<style>
	main {
		text-align: center;
		padding: 1em;
		max-width: 240px;
		margin: 0 auto;
	}

	table {
		margin: auto;
	}

	h1 {
		color: #ff3e00;
		text-transform: uppercase;
		font-size: 4em;
		font-weight: 100;
	}

	@media (min-width: 640px) {
		main {
			max-width: none;
		}
	}
</style>