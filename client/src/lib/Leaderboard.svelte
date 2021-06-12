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
</script>

<div>
	<Section name={label || $t('panel.leaderboard.label')} {open}>
		<table>
			{#each leaderboard as {name, team, score}}
				<tr>
					<td class='name'>{team ? `[${team}] ${name}` : name}</td>
					<td class='score'>{score || 0}</td>
				</tr>
			{/each}
		</table>
	</Section>
</div>

<style>
	div {
		background-color: #00000040;
		margin: 10px;
		padding: 10px;
	}

	h2 {
		margin-bottom: 10px;
		margin-top: 0px;
	}

	table {
		width: 100%;
	}

	td.name {
		font-weight: bold;
	}

	td.score {
		text-align: right;
	}
</style>
