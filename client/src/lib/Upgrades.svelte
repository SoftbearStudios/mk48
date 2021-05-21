<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from './Section.svelte';
	import entityData from '../data/entities.json';
	import {clamp} from '../util/math.js';

	export let score;
	export let type;
	export let callback;

	function levelToScore(level) {
		return (level * level - 1) * 10;
	}

	$: level = entityData[type].level;
	$: nextLevel = level + 1;
	$: progress = clamp(((score || 0) - levelToScore(level)) / (levelToScore(nextLevel) - levelToScore(level)), 0, 1);
	$: ready = progress === 1;

	function upgrades(nextLevel) {
		const list = [];
		for (const entityType of Object.keys(entityData)) {
			const data = entityData[entityType];
			if (data.type === 'boat' && data.level === nextLevel) {
				list.push(entityType);
			}
		}
		return list;
	}
</script>

<div>
	<Section name={`${Math.round(progress * 100)}% to level ${nextLevel}`}>
		{#each upgrades(nextLevel) as upgradeType}
			<img title={`${entityData[upgradeType].label} (${entityData[upgradeType].subtype})`} class:ready on:click={() => ready ? callback(upgradeType) : null} alt={upgradeType} src={`/sprites/${upgradeType}.png`}/>
			<br/>
		{:else}
			<p>Maximum ship level reached</p>
		{/each}
	</Section>
</div>

<style>
	div {
		background-color: #00000040;
		left: 0;
		margin: 10px;
		padding: 10px;
		position: absolute;
		top: 0;
	}

	h2 {
		margin-bottom: 0px;
		margin-top: 0px;
	}

	img {
		margin: 5px;
		max-width: 15em;
	}

	img:not(.ready) {
		opacity: 0.6;
	}

	p {
		margin-bottom: 0;
	}
</style>
