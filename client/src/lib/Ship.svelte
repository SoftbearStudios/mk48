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
		const aED = entityData[armamentData.type];
		return `${aED.kind}/${aED.subkind}`;
	}

	export function groupArmaments(armaments, consumptions) {
		const groups = {};
		for (let i = 0; i < armaments.length; i++) {
			const armament = armaments[i];

			const type = getArmamentType(armament);

			let group = groups[type];
			if (!group) {
				group = {type: armament.type, ready: 0, deployed: 0, total: 0, reload: 0};
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
		let subtype = data.subkind;
		if (subtype === 'rocket' && data.armaments && data.armaments.length > 0) {
			subtype = 'rocketTorpedo';
		}
		return translation(`kind.${data.kind}.${subtype}.name`);
	}

	// Returns null if no sensors
	function getActiveSensorHint(translate, type, altitude = 0) {
		const data = entityData[type];
		const sensors = data.sensors;
		if (!sensors) {
			return null;
		}
		let list = [];
		if (sensors.radar && sensors.radar.range > 0 && altitude >= 0) {
			list.push('radar');
		}
		if (sensors.sonar && sensors.sonar.range > 0) {
			list.push('sonar');
		}
		if (list.length == 0) {
			return null;
		}

		list = list.map(type => translate(`sensor.${type}.label`));
		let hint = translate('panel.ship.action.active.hint');
		hint = hint.replace('{sensors}', list.join(' / '));
		return hint;
	}
</script>

<script>
	import Section from './Section.svelte';
	import entityData from '../data/entities.json';

	export let type;
	export let altitude;
	export let consumption;
	export let selection = null;
	export let altitudeTarget = 0;
	export let active = true;

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

	export function toggleActive() {
		active = !active;
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
		{#if entityData[type].subkind === 'submarine'}
			<div class='button' class:selected={altitudeTarget === 0} on:click={toggleAltitudeTarget}>{$t('panel.ship.action.surface.label')}</div>
		{/if}
		{#if getActiveSensorHint($t, type, altitude)}
			<div class='button' class:selected={active} on:click={toggleActive} title={getActiveSensorHint($t, type, altitude)}>{$t(`panel.ship.action.active.label`)}</div>
		{/if}
		{#if !armaments || armaments.length === 0}
			<small>{$t(`kind.boat.${entityData[type].subkind}.hint`)}</small>
		{/if}
	</Section>
</div>

<style>
	div.container {
		max-width: 25%;
	}

	.button {
		padding: 5px;
		filter: brightness(0.8);
		user-select: none;
	}

	.button:hover {
		background-color: #44444440;
		filter: brightness(0.9);
	}

	.button.selected {
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

	@media(max-width: 800px) {
		span.consumption {
			display: none;
		}
	}
</style>
