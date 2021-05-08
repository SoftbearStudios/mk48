<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {plural} from '../util/strings.js';
	import {fly} from 'svelte/transition';

	export let overlay;

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
	<h2>{overlay.score || 0} {plural('point', overlay.score || 0)} —
	{(overlay.speed * 1.943844492).toFixed(1)}kn —
	{Math.round(((overlay.direction + Math.PI / 2) * 180 / Math.PI % 360 + 360) % 360)}° [{directionString(overlay.direction)}] —
	({positionString(overlay.positionX, 'E', 'W')}, {positionString(overlay.positionY, 'S', 'N')})</h2>
</div>

<style>
	div {
		bottom: 0;
		left: 0;
		pointer-events: none;
		position: absolute;
		right: 0;
		text-align: center;
		user-select: none;
	}
</style>
