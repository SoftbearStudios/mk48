<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';
	import {onMount} from 'svelte';
	import {fade} from 'svelte/transition';
	import t, {setLanguage, translateAs} from './translation.js';
	import strings from '../data/strings.json';
	import {onDestroy} from 'svelte';

	export let type; // boat entity type
	let timeout = null;

	function reset() {
		if (timeout !== null) {
			clearTimeout(timeout);
		}
		timeout = setTimeout(() => {
			timeout = null;
		}, 5000);
	}

	onDestroy(() => {
		if (timeout !== null) {
			clearTimeout(timeout);
			timeout = null;
		}
	});

	$: type && reset(); // reset timer whenever entityType changes
</script>

{#if type && timeout}
	<div class='splash' transition:fade="{{duration: 500}}">
		{$t(`kind.boat.${entityData[type].subtype.toLowerCase()}.hint`)}
	</div>
{/if}

<style>
	div {
		background-color: #00000040;
		color: white;
		height: min-content;
		margin: auto;
		padding: 0.5em;
		position: absolute;
		text-align: center;
		left: 50%;
		top: 65%;
		transform: translate(-50%, -50%);
		opacity: 0.9;
		user-select: none;
		pointer-events: none;
	}
</style>
