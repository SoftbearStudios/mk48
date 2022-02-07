<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Dialog from './Dialog.svelte';
	import entityDatas from '../data/entities.json';
	import EntityCard from '../component/EntityCard.svelte';
	import t from '../util/translation.js';
</script>

<Dialog title={$t('page.ships.title')}>
	<p>{$t('page.ships.intro')}</p>

	<table>
		{#each Object.entries(entityDatas) as [type, entityData]}
			{#if entityData.kind === 'boat'}
				<tr>
					<td>
						<EntityCard {type}/>
					</td>
				</tr>
			{/if}
		{/each}
	</table>
</Dialog>

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
</style>
