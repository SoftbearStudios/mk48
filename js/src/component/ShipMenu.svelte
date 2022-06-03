<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from './Section.svelte';
	import Sprite from './Sprite.svelte';
	import t from '../util/translation.js';
	import {clamp} from '../util/math.js';
	import {availableShips, summarizeType} from '../util/warship.js';
	import entityData from '../data/entities.json';
	import storage from '../util/storage.js';
	import IconButton from './IconButton.svelte';
	import Locked from "svelte-bootstrap-icons/lib/LockFill";
	import Restricted from "svelte-bootstrap-icons/lib/Snow2";
	import Unlocked from "svelte-bootstrap-icons/lib/UnlockFill";
	import {onMount} from 'svelte';

	export let level;
	export let maxLevel;
	export let minLevel = 1;
	export let name;
	export let onSelectShip;
	export let open = true;
	export let restrictions;
	export let type;
	export let closable = true;

	let forcedUnlocks = {};
	const FORCE_UNLOCKS = 5;

	$: level = clamp(level || minLevel, minLevel, maxLevel);
	$: ships = availableShips(level, type);
	$: columns = ships.length > 3;

	onMount(() => {
		level = maxLevel;
	});

	// $: console.log(`min=${minLevel}, max=${maxLevel}, level=${level}`);

	function handleSelectShip(shipType) {
		if (onSelectShip && !(restricted(shipType, restrictions) || locked(shipType, forcedUnlocks))) {
			onSelectShip(shipType);
		}
	}

	function incrementIndex(value) {
		level = clamp(level + value, minLevel, maxLevel);
	}

	// Some ships are difficult/confusing. Lock them until the player has a bit of experience with the game.
	function locked(type, forcedUnlocks) {
		const data = entityData[type];
		const minutesPlayed = (Date.now() - (storage.join || 0)) / (60 * 1000);
		return (forcedUnlocks[type] || 0) < FORCE_UNLOCKS && minutesPlayed < ({dredger: 15, minelayer: 30, icebreaker: 45, tanker: 60}[data.subkind] || -1);
	}

	function restricted(type, restrictions) {
		return restrictions ? restrictions.includes(type) : false;
	}

	// Protest locking of ships (click the lock x times to unlock manually).
	function unlockShip(type) {
		forcedUnlocks[type] = (forcedUnlocks[type] || 0) + 1;
	}
</script>

<Section disableLeftArrow={level == minLevel} disableRightArrow={level == maxLevel} headerAlign='center' name={name} bind:open onLeftArrow={() => incrementIndex(-1)} onRightArrow={() => incrementIndex(1)} {closable}>
	<div class="ships" class:columns={ships.length > 3}>
		{#each ships as shipType}
			<Sprite
				title={`${entityData[shipType].label} (${summarizeType($t, shipType)})`}
				consumed={restricted(shipType, restrictions) || locked(shipType, forcedUnlocks)}
				icon={restricted(shipType, restrictions) ? Restricted : locked(shipType, forcedUnlocks) ? ((forcedUnlocks[shipType] || 0) < FORCE_UNLOCKS - 1 ? Locked : Unlocked) : null}
				iconTitle={restricted(shipType, restrictions) ? 'Cannot choose this ship in this area' : 'New players are not advised to choose this ship'}
				onIconClick={() => unlockShip(shipType)}
				on:click={() => handleSelectShip(shipType)}
				name={shipType}
			/>
		{/each}
	</div>
</Section>

<style>
	div.ships {
		display: grid;
		grid-gap: 1.5rem 1.5rem;
		grid-template-columns: repeat(1, 1fr);
		margin: auto;
		padding-top: 1.5rem;
		user-select: none;
		width: min-content;
		-webkit-user-drag: none;
	}

	@media(min-width: 1000px) {
		div.ships.columns {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media(min-width: 600px) and (max-height: 500px) {
		div.ships {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>
