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
	import {plural} from '../util/strings.js';
	import {fly} from 'svelte/transition';
	import t from './translation.js';

	export let overlay;
	export let recording = false;

	function positionString(element, positiveLabel, negativeLabel) {
		return `${Math.round(Math.abs(element))}${element >= 0 ? positiveLabel : negativeLabel}`
	}

	function directionString(angle) {
		// angle (0 to 1)
		const theta = (angle + Math.PI) / (2 * Math.PI);
		const directions = ['W', 'NW', 'N', 'NE', 'E', 'SE', 'S', 'SW'];
		const index = Math.round(theta * directions.length);
		return directions[((index + directions.length) % directions.length + directions.length) % directions.length];
	}
</script>

<div transition:fly="{{y: 100}}">
	<h2>{overlay.score || 0} {$t('panel.status.score' + (overlay.score === 1 ? '' : 'Plural'))} —
	{toKnotsString(overlay.speed)} —
	{Math.round(((overlay.direction + Math.PI / 2) * 180 / Math.PI % 360 + 360) % 360)}° [{directionString(overlay.direction)}] —
	({positionString(overlay.positionX, 'E', 'W')}, {positionString(overlay.positionY, 'S', 'N')})
	{recording ? ' — Recording (v to stop)' : ''}</h2>
</div>

<style>
	div {
		text-align: center;
		user-select: none;
	}

	h2 {
		margin-bottom: 0.25em;
	}
</style>
