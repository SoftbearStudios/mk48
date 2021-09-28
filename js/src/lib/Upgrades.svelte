<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import entityData from '../data/entities.json';

	function levelToScore(level) {
		// Must match server code
		return (level * level - 1) * 10;
	}

	function getUpgrades(type) {
		const nextLevel = entityData[type].level + 1;
		const list = [];
		for (const entityType of Object.keys(entityData)) {
			const data = entityData[entityType];
			if (data.kind === 'boat' && data.level === nextLevel && !data.npc) {
				list.push(entityType);
			}
		}
		return list;
	}

	export function upgradeProgress(type, score) {
		const level = entityData[type].level;
		return clamp(((score || 0) - levelToScore(level)) / (levelToScore(level + 1) - levelToScore(level)), 0, 1);
	}

	export function hasUpgrades(type) {
		return getUpgrades(type).length > 0;
	}

	export function canUpgrade(type, score) {
		return upgradeProgress(type, score) === 1 && hasUpgrades(type);
	}
</script>

<script>
	import Section from './Section.svelte';
	import Sprite from './Sprite.svelte';
	import t from './translation.js';
	import {clamp} from '../util/math.js';
	import {summarizeType} from './Ship.svelte';

	export let type;
	export let onUpgrade;

	$: upgrades = getUpgrades(type);
	$: columns = upgrades.length > 3;
</script>

<div class='box' class:columns>
	<Section name={`${$t('panel.upgrade.labelPrefix')} ${entityData[type].level + 1}`} headerAlign='center'>
		<div class='upgrades' class:columns>
			{#each upgrades as upgradeType}
				<Sprite title={`${entityData[upgradeType].label} (${summarizeType($t, upgradeType)})`} on:click={onUpgrade.bind(null, upgradeType)} name={upgradeType}/>
			{/each}
		</div>
	</Section>
</div>

<style>
	div.box {
		width: min-content;
		min-width: 15%;
	}

	div.box.columns {
		min-width: 30%
	}

	div.upgrades {
		padding: 1em;
		display: grid;
		grid-gap: 1em 1em;
		grid-template-columns: repeat(1, 1fr);
		user-select: none;
		width: min-content;
		margin: auto;
		-webkit-user-drag: none;
	}

	@media(min-width: 1000px) {
		div.upgrades.columns {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>
