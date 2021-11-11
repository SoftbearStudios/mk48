<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import entityData from '../data/entities.json';

	function levelToScore(level) {
		// Must match server code
		return (level * level + Math.pow(2, Math.max(level - 3, 0)) - 2) * 10;
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
	import storage from '../util/storage.js';
	import Locked from "svelte-bootstrap-icons/lib/LockFill";
	import Unlocked from "svelte-bootstrap-icons/lib/UnlockFill";

	export let type;
	export let onUpgrade;

	$: data = entityData[type];

	// Protest locking of ships (click the lock x times to unlock manually).
	let forceUnlocks = {};
	const FORCE_UNLOCKS = 5;
	function forceUnlock(type) {
		forceUnlocks[type] = (forceUnlocks[type] || 0) + 1;
	}

	// Some ships are difficult/confusing. Lock them until the player has a bit of experience with the game.
	function locked(type, forceUnlocks) {
		const data = entityData[type];
		const minutesPlayed = (Date.now() - (storage.join || 0)) / (60 * 1000);
		return (forceUnlocks[type] || 0) < FORCE_UNLOCKS && minutesPlayed < ({minelayer: 30, 'dredger': 15, 'tanker': 60}[data.subkind] || -1);
	}

	$: upgrades = getUpgrades(type);
	$: columns = upgrades.length > 3;
</script>

<div class='box' class:columns>
	<Section name={`${$t('panel.upgrade.labelPrefix')} ${data.level + 1}`} headerAlign='center'>
		<div class='upgrades' class:columns>
			{#each upgrades as upgradeType}
				<Sprite
					title={`${entityData[upgradeType].label} (${summarizeType($t, upgradeType)})`}
					consumed={locked(upgradeType, forceUnlocks)}
					icon={locked(upgradeType, forceUnlocks) ? ((forceUnlocks[upgradeType] || 0) < FORCE_UNLOCKS - 1 ? Locked : Unlocked) : null}
					iconTitle={'New players are not advised to choose this ship'}
					onIconClick={() => forceUnlock(upgradeType)}
					on:click={locked(upgradeType, forceUnlocks) ? null : onUpgrade.bind(null, upgradeType)}
					name={upgradeType}
				/>
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
