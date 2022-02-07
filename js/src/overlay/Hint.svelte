<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';
	import {onMount} from 'svelte';
	import {cinematic} from '../util/settings.js';
	import {fade} from 'svelte/transition';
	import t, {setLanguage, translate} from '../util/translation.js';
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

{#if type && timeout && !$cinematic}
	<div id='hint' transition:fade="{{duration: 500}}">
		{$t(`kind.boat.${entityData[type].subkind.toLowerCase()}.hint`)}
	</div>
{/if}

<style>
	#hint {
		background-color: #00000040;
		color: white;
		height: min-content;
		left: 50%;
		margin: auto;
		opacity: 0.9;
		padding: 0.5em;
		pointer-events: none;
		position: absolute;
		text-align: center;
		top: 65%;
		transform: translate(-50%, -50%);
		user-select: none;
	}
</style>
