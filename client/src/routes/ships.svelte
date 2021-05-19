<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityDatas from '../data/entities.json';
	import EntityCard from '../lib/EntityCard.svelte';
</script>

<div>
	<h1>Mk48.io Ships</h1>

	<p>The following is a list of all ships in the game, and their weapons.</p>

	<table>
		{#each Object.entries(entityDatas) as [type, entityData]}
			{#if entityData.type === 'boat'}
				<tr>
					<td>
						<EntityCard {type}/>
					</td>
				</tr>
			{/if}
		{/each}
	</table>
</div>

<style>
	div {
		background-color: #2c3e50;
		color: white;
		font-family: sans-serif;
		font-size: 20px;
		overflow-y: scroll;
		padding: 10px;
		position: absolute;
		inset: 0;
	}

	a {
		color: white;
	}

	h1, h2, h3 {
		margin-bottom: 0.5em;
		margin-top: 0;
	}

	table {
		border-spacing: 1em;
		text-align: left;
		width: 100%;
	}

	table.item {
		background-color: #00000011;
	}

	img {
		width: 30em;
	}
</style>
