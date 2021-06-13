<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import {fromCamelCase} from '../util/strings.js';
	import t from './translation.js';

	function armamentConsumption(consumption, index) {
		return (!consumption || consumption.length <= index) ? 0 : consumption[index]
	}

	export function hasArmament(consumption, index) {
		return armamentConsumption(consumption, index) < 0.001;
	}

	export function getArmamentType(armamentData) {
		const armamentType = armamentData.type || entityData[armamentData.default].type;
		const armamentSubtype = armamentData.subtype || entityData[armamentData.default].subtype;
		return `${armamentType}/${armamentSubtype}`;
	}

	export function groupArmaments(armaments, consumptions) {
		const groups = {};
		for (let i = 0; i < armaments.length; i++) {
			const armament = armaments[i];

			const armamentType = armament.type || entityData[armament.default].type;
			const armamentSubtype = armament.subtype || entityData[armament.default].subtype;

			const type = getArmamentType(armament);

			let group = groups[type];
			if (!group) {
				group = {type: armament.default, ready: 0, deployed: 0, total: 0, reload: 0};
				groups[type] = group;
			}
			group.total++;

			let consumption = armamentConsumption(consumptions, i);
			if (consumption >= 6553) {
				group.deployed++;
			} else if (consumption > 0) {
				group.reload += consumption;
			} else {
				group.ready++;
			}
		}

		return Object.entries(groups);
	}

	export function summarizeType(translation, type) {
		const data = entityData[type];
		let subtype = data.subtype;
		if (subtype === 'MTB') {
			// TODO: Temporary patch
			subtype = 'mtb';
		}
		if (subtype === 'rocket' && data.armaments && data.armaments.length > 0) {
			subtype = 'rocketTorpedo';
		}
		return translation(`kind.${data.type}.${subtype}.name`);
	}
</script>

<script>
	import Section from './Section.svelte';
	import entityData from '../data/entities.json';

	export let type;
	export let consumption;
	export let selection = null;
	export let altitudeTarget = 0;

	$: armaments = entityData[type].armaments;
	$: armaments && incrementSelection(0); // make sure a valid armament is selected

	export function incrementSelection(increment = 1) {
		const groups = groupArmaments(armaments, consumption);
		if (groups.length === 0) {
			selection = null;
		} else {
			let currentIndex = groups.findIndex(([type, armament]) => type === selection);
			if (currentIndex == -1) {
				currentIndex = 0;
			}
			currentIndex = (currentIndex + increment + groups.length) % groups.length;
			selection = groups[currentIndex][0];
		}
	}

	export function setSelectionIndex(index) {
		const groups = groupArmaments(armaments, consumption);
		if (index < 0 || index >= groups.length) {
			selection = null;
		} else {
			selection = groups[index][0];
		}
	}

	export function toggleAltitudeTarget() {
		if (altitudeTarget === 0) {
			altitudeTarget = -1;
		} else {
			altitudeTarget = 0;
		}
	}
</script>

<div class='container'>
	<Section name={entityData[type].label}>
		{#each groupArmaments(armaments, consumption) as [type, group]}
			<div class='button' class:selected={type === selection} on:click={() => selection = type}>
				<img title={`${entityData[group.type].label} (${summarizeType($t, group.type)})`} class:consumed={group.ready === 0} src={`/entities/${group.type}.png`}/>
				<span class='consumption' title={(group.reload === 0 ? 'Fully reloaded' : `${Math.round(group.reload)}s to full reload`) + (group.deployed === 0 ? '' : ` (${group.deployed} still deployed)`)}>{group.ready}/{group.total}</span>
			</div>
		{/each}
		{#if entityData[type].subtype === 'ram'}
			<small>{$t('kind.boat.ram.hint')}</small>
		{:else if entityData[type].subtype === 'submarine'}
			<div class='button' class:selected={altitudeTarget === 0} on:click={toggleAltitudeTarget}>Surface</div>
		{/if}
	</Section>
</div>

<style>
	div.container {
		max-width: 25%;
	}

	div.button {
		padding: 5px;
		filter: brightness(0.8);
		user-select: none;
	}

	div.button:hover {
		background-color: #44444440;
		filter: brightness(0.9);
	}

	div.button.selected {
		background-color: #44444480;
		filter: brightness(1.2);
		padding: 5px;
	}

	div.button:not(.selected) {
		cursor: pointer;
	}

	h2 {
		margin-bottom: 10px;
		margin-top: 0px;
	}

	table {
		width: 100%;
		border-spacing: 10px;
	}

	img {
		max-height: 40px;
		max-width: 100px;
	}

	img.consumed {
		opacity: 0.6;
	}

	span.consumption {
		float: right;
		color: white;
	}
</style>
