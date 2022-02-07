<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {onMount} from 'svelte';
	import entityData from '../data/entities.json';
	import ShipMenu from '../component/ShipMenu.svelte';
	import t from '../util/translation.js';
	import {summarizeType} from '../util/warship.js';
	import storage from '../util/storage.js';

	export let onSpawn;
	export let respawnLevel;
	export let state;

	let level;
	let paused = false;

	onMount(() => {
		sendToParent('splash');
	});

	function fmtDeathReason(t, reason) {
		let message = t(`death.${reason.type}.message`).replace('{player}', reason.player);
		let entityKind = undefined;
		let entityLabel = undefined;
		if (reason.entity) {
			entityKind = summarizeType(t, reason.entity);
			entityLabel = entityData[reason.entity].label;
		}
		message = message.replace('{entityKind}', entityKind)
		message = message.replace('{playerOrEntityLabel}', reason.player || entityLabel);

		return message;
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

	function handleRespawn(boatType) {
		if (onSpawn) {
			let name = storage.name || '';
			onSpawn(name, boatType);
		}
		sendToParent('play');
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

<div id='respawn_overlay'>
	{#if state.status.respawning}
		<h2 class='reason'>{fmtDeathReason($t, state.status.respawning.deathReason)}</h2>
	{/if}
	<div class='respawn_menu'>
		{#if !paused}
			<ShipMenu bind:level={level} maxLevel={respawnLevel} minLevel={1} name={($t('panel.respawn.label')).replace("{level}", level)} onSelectShip={handleRespawn} onClickSection={() => false}/>
		{/if}
	</div>
</div>

<svelte:window on:message={handleMessage}/>

<style>
	#respawn_overlay {
		left: 50%;
		min-width: 30%;
		padding-top: 1rem;
		position: absolute;
		top: 5%;
		transform: translate(-50%, 0);
		width: min-content;
	}

	div.respawn_menu {
		display: grid;
		grid-gap: 1em 1em;
		grid-template-columns: repeat(1, 1fr);
		margin: auto;
		padding: 1em;
		user-select: none;
		width: min-content;
		-webkit-user-drag: none;
	}

	h2 {
		color: white;
		font-weight: bold;
		margin: 0;
		text-align: center;
		transition: filter 0.1s;
		user-select: none;
	}
</style>
