<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from './Section.svelte';
	import t from './translation.js';

	export let leaderboard;
	export let label = null;
	export let open = true;
	export let headerAlign = 'left';
	export let footer;
</script>

<Section name={label || $t('panel.leaderboard.label')} {headerAlign} {open}>
	<table>
		{#each leaderboard as {name, team, score}}
			<tr>
				<td class='name'>{team ? `[${team}] ${name}` : name}</td>
				<td class='score'>{score || 0}</td>
			</tr>
		{/each}
	</table>
	{#if footer}
		<p>{footer}</p>
	{/if}
</Section>

<style>
	table {
		width: 100%;
		color: white;
	}

	td.name {
		font-weight: bold;
	}

	td.score {
		text-align: right;
	}

	p {
		color: white;
		text-align: center;
		margin: 0.5em;
		font-style: italic;
	}
</style>
