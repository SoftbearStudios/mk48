<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Page from '../lib/Page.svelte';
	import entityDatas from '../data/entities.json';
	import EntityCard from '../lib/EntityCard.svelte';
</script>

<Page>
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
</Page>

<style>
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
