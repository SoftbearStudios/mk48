<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from '../component/Section.svelte';
	import entityData from '../data/entities.json';
	import Sprite from '../component/Sprite.svelte';
	import {cinematic, shipControlsShown} from '../util/settings.js';
	import {fromCamelCase} from '../util/strings.js';
	import {hasArmament, getArmamentType, groupArmaments, summarizeType} from '../util/warship.js';
	import t from '../util/translation.js';

	export let state;
	export let selection = null;
	export let altitudeTarget = 0;
	export let active = true;

	$: alive = state.status.playing;
	$: armaments = entityData[alive.type].armaments;
	$: armaments && incrementSelection(0); // make sure a valid armament is selected

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

	export function incrementSelection(increment = 1) {
		const groups = groupArmaments(armaments, alive.armamentConsumption);
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
		const groups = groupArmaments(armaments, alive.armamentConsumption);
		if (index < 0) {
			selection = null;
		} else if (index < groups.length) {
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

<div id='ship_controls' class:cinematic={$cinematic}>
	<Section name={entityData[alive.type].label} bind:open={$shipControlsShown}>
		{#each groupArmaments(armaments, alive.armamentConsumption) as [type, group]}
			<div class='button' class:selected={type === selection} on:click={() => selection = type}>
				<Sprite title={`${entityData[group.type].label} (${summarizeType($t, group.type)})`} consumed={group.ready === 0} name={group.type}/>
				<span class='consumption'>{group.ready}/{group.total}</span>
			</div>
		{/each}
		{#if entityData[alive.type].subkind === 'submarine'}
			<div class='button' class:selected={altitudeTarget === 0} on:click={toggleAltitudeTarget} title={$t(`panel.ship.action.surface.hint`)}>{$t('panel.ship.action.surface.label')}</div>
		{/if}
		{#if getActiveSensorHint($t, alive.type, alive.altitude)}
			<div class='button' class:selected={active} on:click={toggleActive} title={getActiveSensorHint($t, alive.type, alive.altitude)}>{$t(`panel.ship.action.active.label`)}</div>
		{/if}
		{#if !armaments || armaments.length === 0}
			<small>{$t(`kind.boat.${entityData[alive.type].subkind}.hint`)}</small>
		{/if}
	</Section>
</div>

<style>
	#ship_controls {
		bottom: 0;
		left: 0;
		max-width: 25%;
		padding-bottom: 1rem;
		padding-left: 1rem;
		position: absolute;
	}

	.button {
		color: white;
		padding: 0.5em;
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
		padding: 0.5em;
	}

	div.button:not(.selected) {
		cursor: pointer;
	}

	div.cinematic:not(:hover) {
		opacity: 0;
	}

	span.consumption {
		float: right;
		color: white;
	}

	small {
		color: white;
	}

	@media(max-width: 800px) {
		span.consumption {
			display: none;
		}
	}
</style>
