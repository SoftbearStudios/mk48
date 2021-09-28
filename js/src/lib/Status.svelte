<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	export function toKnotsString(speed) {
		return `${((speed || 0) * 1.943844492).toFixed(1)}kn`;
	}
</script>

<script>
	import Meter from './Meter.svelte';
	import t from './translation.js';
	import {fly} from 'svelte/transition';
	import {hasUpgrades, upgradeProgress} from './Upgrades.svelte';
	import entityData from '../data/entities.json';

	export let state;
	$: alive = state.status.alive;

	function positionString(element, positiveLabel, negativeLabel) {
		return `${Math.round(Math.abs(element))}${element >= 0 ? positiveLabel : negativeLabel}`
	}

	function directionString(angle) {
		// angle (0 to 1)
		const theta = (angle + Math.PI) / (2 * Math.PI);
		const directions = ['W', 'SW', 'S', 'SE', 'E', 'NE', 'N', 'NW'];
		const index = Math.round(theta * directions.length);
		return directions[((index + directions.length) % directions.length + directions.length) % directions.length];
	}

	$: progress = upgradeProgress(alive.type, state.score || 0);
</script>

<div transition:fly="{{y: 100}}">
	<h2>
		{state.score || 0} {$t('panel.status.score' + (state.score === 1 ? '' : 'Plural'))} —
		{toKnotsString(alive.velocity)} —
		{Math.round(((alive.direction + Math.PI / 2) * 180 / Math.PI % 360 + 360) % 360)}° [{directionString(alive.direction)}] —
		({positionString(alive.position.x, 'E', 'W')}, {positionString(alive.position.y, 'N', 'S')})
	</h2>
	{#if hasUpgrades(alive.type)}
		<Meter value={progress}>{Math.round(progress * 100)}% {$t('panel.upgrade.labelMiddle')} {entityData[alive.type].level + 1}</Meter>
	{/if}
</div>

<style>
	div {
		text-align: center;
		user-select: none;
	}

	h2 {
		color: white;
		margin-bottom: 0.25em;
	}
</style>
