<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from './Section.svelte';
	import t from './translation.js';
	import entityData from '../data/entities.json';
	import {clamp} from '../util/math.js';
	import {summarizeType} from './Ship.svelte';

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

	function getUpgrades(nextLevel) {
		const list = [];
		for (const entityType of Object.keys(entityData)) {
			const data = entityData[entityType];
			if (data.type === 'boat' && data.level === nextLevel && !data.npc) {
				list.push(entityType);
			}
		}
		return list;
	}

	$: upgrades = getUpgrades(nextLevel);
</script>

{#if upgrades && upgrades.length > 0}
	<div>
		<Section name={`${Math.round(progress * 100)}% ${$t('panel.upgrade.labelMiddle')} ${nextLevel}`}>
			{#each upgrades as upgradeType}
				<img title={`${entityData[upgradeType].label} (${summarizeType($t, upgradeType)})`} class:ready on:click={() => ready ? callback(upgradeType) : null} alt={upgradeType} src={`/entities/${upgradeType}.png`}/>
				<br/>
			{/each}
		</Section>
	</div>
{/if}

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
