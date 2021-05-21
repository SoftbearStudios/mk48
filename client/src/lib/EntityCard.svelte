<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityDatas from '../data/entities.json';
	import Link from './Link.svelte';
	import {groupArmaments} from './Ship.svelte';
	import {fromCamelCase} from '../util/strings.js';

	export let type;
	let entityData;
	$: {
		entityData = entityDatas[type];
		if (!entityData) {
			throw Error(`unknown entity type: ${type}`);
		}
	}
</script>

<table class='item'>
	<tr>
		<td>
			<h3>{entityData.label}</h3>
			{#if entityData.type === 'boat'}
				<i>Level {entityData.level} {entityData.subtype}</i>
			{:else}
				<i>{fromCamelCase(entityData.subtype)} {entityData.type === 'weapon' ? '' : entityData.type}</i>
			{/if}
			{#if entityData.link}
				(<Link href={entityData.link}>Learn more</Link>)
			{/if}
		</td>
		<td rowspan={2}>
			<ul>
				{#if entityData.length}
					<li>Length: {entityData.length.toFixed(1)}m</li>
				{/if}
				{#if entityData.speed}
					<li>Speed: {entityData.speed.toFixed(1)}m/s</li>
				{/if}
				{#if entityData.range}
					<li>Range: {entityData.range.toFixed(0)}m</li>
				{/if}
				{#if entityData.lifespan}
					<li>Lifespan: {entityData.lifespan.toFixed(1)}s</li>
				{/if}
				{#if entityData.reload}
					<li>Reload: {entityData.reload.toFixed(1)}s</li>
				{/if}
				{#if entityData.damage}
					<li>Damage: {entityData.damage.toFixed(1)}</li>
				{/if}
				{#if entityData.stealth}
					<li>Stealth: {Math.round(entityData.stealth * 100)}%</li>
				{/if}
			</ul>
		<td/>
	</tr>
	<tr>
		<td>
			<img class:ship={entityData.type === 'boat'} title={entityData.label} alt={type} src={`/sprites/${type}.png`}/>
		</td>
	</tr>
	{#each groupArmaments(entityData.armaments, []) as [type, group]}
		<tr>
			<td colspan={2}>
				<svelte:self type={group.type}/>
			</td>
		</tr>
	{/each}
</table>

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
		table-layout: fixed;
		text-align: left;
		width: 100%;
	}

	table.card {
		background-color: #00000011;
	}

	td {
		text-align: left;
	}

	img {
		width: 100%;
		max-height: 5em;
		object-fit: contain;
	}
</style>
