<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {fade, fly} from 'svelte/transition';
	import t from './translation.js';

	export let touch;
	export let instructBasics = true;
	export let instructZoom = true;
</script>

{#if instructBasics}
	<div in:fly="{{y: -100}}" out:fade>
		{#if touch}
			<h2>{$t('panel.instruction.touch.movement')}</h2>
			{#if instructZoom}
				<p out:fade>{$t('panel.instruction.touch.zoom')}</p>
			{/if}
		{:else}
			<h2>{$t('panel.instruction.mouse.movement')}</h2>
			{#if instructZoom}
				<p out:fade>{$t('panel.instruction.mouse.zoom')}</p>
			{/if}
		{/if}
	</div>
{/if}

<style>
	div {
		pointer-events: none;
		text-align: center;
		user-select: none;
	}
</style>
