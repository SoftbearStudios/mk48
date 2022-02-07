<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import IconButton from '../component/IconButton.svelte';
	import Section from '../component/Section.svelte';
	import {cinematic, leaderboardShown} from '../util/settings.js';
	import t from '../util/translation.js';

	export let footer;
	export let state;

	let leaderboardIndex;
	const LEADERBOARD_PROPERTIES = ["Daily", "Weekly", "AllTime"];
	const LEADERBOARD_TYPES = ["single/day", "single/week", "single/all"];
	$: leaderboardContent = leaderboardIndex === undefined ? state.liveboard : state.leaderboards[LEADERBOARD_PROPERTIES[leaderboardIndex]];
	// The $t parameter is not used but it causes the function to be re-evaluated when $t changes.
	$: leaderboardName = getLeaderboardName(leaderboardIndex, $t);

	function handleCycleLeaderboard() {
		leaderboardIndex = getNextIndex(leaderboardIndex);
	}

	function getLeaderboardName(index) {
		return	index === undefined ?
				$t('panel.leaderboard.label') :
				$t('panel.leaderboard.type.' + LEADERBOARD_TYPES[index]);
	}

	function getNextIndex(index) {
		if (index === undefined) {
			return 0;
		} else if (index == LEADERBOARD_TYPES.length - 1) {
			return undefined;
		} else {
			return index + 1;
		}
	}
</script>

<div id="leaderboard" class:cinematic={$cinematic}>
	<Section name={leaderboardName} headerAlign={'right'} onRightArrow={handleCycleLeaderboard} bind:open={$leaderboardShown}>
		<table>
			{#each leaderboardContent as {name, team, score}}
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
</div>

<style>
	div.cinematic:not(:hover) {
		opacity: 0;
	}

	#leaderboard {
		max-width: 25%;
		padding-right: 1rem;
		padding-top: 1rem;
		position: absolute;
		right: 0;
		text-align: right;
		top: 0;
	}

	p {
		color: white;
		font-style: italic;
		margin-bottom: 1.4rem;
		margin-top: 0.5rem;
		text-align: center;
	}

	table {
		color: white;
		width: 100%;
	}

	td.name {
		font-weight: bold;
		text-align: left;
	}

	td.score {
		text-align: right;
	}
</style>
