<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {fade, fly} from 'svelte/transition';

	export let touch;
	export let instructBasics = true;
	export let instructZoom = true;
</script>

{#if instructBasics}
	<div in:fly="{{y: -100}}" out:fade>
		{#if touch}
			<h2>Touch in a direction to move, tap to fire torpedos</h2>
		{:else}
			<h2>Click and hold to move, click to fire torpedos</h2>
			{#if instructZoom}
				<p out:fade>Scroll to zoom out for a better view</p>
			{/if}
		{/if}
	</div>
{/if}

<style>
	div {
		left: 0;
		pointer-events: none;
		position: absolute;
		right: 0;
		text-align: center;
        top: 0;
		user-select: none;
	}
</style>
