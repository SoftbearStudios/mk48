<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';
	import storage from '../util/storage.js';
	import {cinematic} from '../util/settings.js';
	import {fade} from 'svelte/transition';
	import {onMount} from 'svelte';
	import t from '../util/translation.js';
	import strings from '../data/strings.json';
	import Logo from '../component/Logo.svelte';

	export let onSpawn;

	let name = storage.name || '';
	let paused = false;
	let transition = $cinematic ? {duration: 500, delay: 5000} : {duration: 500, delay: 1000};
	let transitioning = false;

	const DEFAULT_BOAT_TYPE = "g5";
	const MAX_NAME_LENGTH = 12;
	const MIN_NAME_LENGTH = 1;

	onMount(() => {
		sendToParent('splash');
	});

	// Returns a list of boats suitable for the initial spawn.
	function getSpawnable() {
		const list = [];
		for (const entityType of Object.keys(entityData)) {
			const data = entityData[entityType];

			if (data.kind === 'boat' && (data.level === 1 && !data.npc)) {
				list.push(entityType);
			}
		}
		return list;
	}

	// Message originates in iframe parent. Also handled in App.svelte.
	function handleMessage(event) {
		switch (event.data) {
			case 'pause':
				paused = true;
				break;
			case 'unpause':
				paused = false;
				break;
		}
	}

	function handlePlayClicked() {
		if (!(paused || transitioning)) {
			if (name) {
				storage.name = name;
			}
			if (onSpawn) {
				onSpawn(name, DEFAULT_BOAT_TYPE);
			}
			if (storage.join == undefined) {
				storage.join = Date.now();
			}

			sendToParent('play');
		}
	}


	// Sends a message to the parent of this iframe, usually
	// a game distribution website shim.
	function sendToParent(msg) {
		if (window.parent && window.parent.postMessage) {
			try {
				window.parent.postMessage(msg, '*');
			} catch (err) {
				console.warn(err);
			}
		}
	}


</script>
<div id="spawn_overlay" in:fade={transition} on:introstart={() => transitioning = true} on:introend={() => transitioning = false}>
	<Logo/>
	<input id="alias_input" disabled={paused || transitioning} type='text' name='name' placeholder={$t('panel.splash.action.alias.label')} autocomplete='off' minlength={MIN_NAME_LENGTH} maxlength={MAX_NAME_LENGTH} on:keyup={e => {if (e.keyCode == 13) handlePlayClicked(); }} bind:value={name}/>
	<button id="play_button" disabled={paused || transitioning} on:click={handlePlayClicked}>{$t('panel.splash.action.spawn.label')}</button>
</div>

<svelte:window on:message={handleMessage}/>

<style>
	button {
		background-color: #549f57;
		border-radius: 1rem;
		color: white;
		font-size: 4rem;
		min-width: 15rem;
		left: 50%;
		padding-bottom: 0.7rem;
		padding-top: 0.5rem;
		position: relative;
		transform: translate(-50%, 0%);
		width: min-content;
	}

	div {
		display: flex;
		flex-direction: column;
		font-size: 2rem;
		left: 50%;
		position: absolute;
		row-gap: 2rem;
		top: 50%;
		transform: translate(-50%, -50%);
		user-select: none;
		width: 50%;
	}

	input {
		border-radius: 3rem;
		color: #FFFA;
		padding-left: 2rem;
		text-align: center;
		width: 100%;
	}
</style>
