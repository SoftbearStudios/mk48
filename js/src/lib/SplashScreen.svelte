<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';
	import storage from '../util/storage.js';
	import {onMount} from 'svelte';
	import {fade} from 'svelte/transition';
	import t, {setLanguage, translateAs} from './translation.js';
	import strings from '../data/strings.json';
	import Link, {outboundEnabled} from './Link.svelte';
	import {summarizeType} from './Ship.svelte';

	export let state;
	export let onSpawn;

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

	function getRandomSpawnable() {
		const spawnable = getSpawnable();
		return spawnable[Math.floor(Math.random() * spawnable.length)];
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

	onMount(() => {
		sendToParent('splash');
	});

	let name = storage.name || '';
	let type = storage.type || getRandomSpawnable();
	let invite = undefined;
	let paused = false;

	const minNameLength = 3;
	const maxNameLength = 12;

	function handleSubmit() {
		if (name) {
			storage.name = name;
		}
		storage.type = type;
		onSpawn(name || 'Guest', type);

		if (storage.join == undefined) {
			storage.join = Date.now();
		}

		sendToParent('play');
	}

	// Message originates in iframe parent. Also handled in App.svelte.
	function onMessage(event) {
		switch (event.data) {
			case 'pause':
				paused = true;
				break;
			case 'unpause':
				paused = false;
				break;
		}
	}

	function fmtDeathReason(t, reason) {
		let message = t(`death.${reason.type}.message`);

		message = message.replace('{player}', reason.player);
		//message = message.replace('{entity}', reason.entity);
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
</script>

<div class='splash' in:fade="{{duration: 500, delay: 1000}}">
	<h2>{$t('panel.splash.label')}</h2>
	{#if state.status.spawning.connectionLost}
		<p>{$t('panel.splash.connectionLost')}</p>
	{:else if state.status.spawning.deathReason}
		<p>{fmtDeathReason($t, state.status.spawning.deathReason)}</p>
	{/if}
	<!--<small>Server maintainance in progress</small>-->
	<form on:submit|preventDefault|stopPropagation={handleSubmit}>
		<input type='text' name='name' placeholder='Nickname' autocomplete='off' minlength={minNameLength} maxlength={maxNameLength} bind:value={name}/>
		<select bind:value={type}>
			{#each getSpawnable() as type}
				<option value={type}>{entityData[type].label}</option>
			{/each}
		</select>
		{#if state.status.spawning.connectionLost}
			<button disabled={paused} on:click={() => location.reload(true)}>Reload</button>
		{:else}
			<button disabled={!type || (name && (name.length < minNameLength || name.length > maxNameLength)) || paused}>{$t('panel.splash.action.spawn.label')}</button>
		{/if}
		{#if invite}
			<small>{$t('panel.splash.invitePrefix')} {invite}</small>
		{/if}
	</form>
	<span>
		<a href='#/help'>{$t('panel.splash.action.help.label')}</a>
		<a href='#/about'>{$t('panel.splash.action.about.label')}</a>
		<a href='#/privacy'>{$t('panel.splash.action.privacy.label')}</a>
		<a href='#/terms'>{$t('panel.splash.action.terms.label')}</a>
	</span>
</div>

<div class='translation' in:fade="{{duration: 500, delay: 1000}}">
	<h3>Language</h3>
	<select value={storage.language} on:change={e => setLanguage(e.target.value)}>
		{#each Object.keys(strings) as lang}
			{#if Object.keys(strings[lang]).length > 0}
				<option value={lang}>{translateAs(lang, 'label')}</option>
			{/if}
		{/each}
	</select>
</div>

<svelte:window on:message={onMessage}/>

<style>
	div {
		background-color: white;
		border-radius: 0.5em;
		box-shadow: 0em 0.2em 0 #cccccc;
		color: black;
		height: min-content;
		margin: auto;
		padding: 0.8em;
		position: absolute;
		text-align: center;
		width: min-content;
	}

	div.splash {
		left: 50%;
		top: 50%;
		transform: translate(-50%, -50%);
	}

	div.translation {
		right: 1em;
		bottom: 1em;
	}

	h2, h3 {
		margin: 0;
		color: black;
	}

	form {
		padding-bottom: 1em;
	}

	button {
		font-weight: bold;
	}

	input, select {
		border: 1px solid gray;
		color: black;
		cursor: pointer;
		margin-top: 0.5em;
		min-width: 16em;
		outline: 0;
		padding: 0.5em;
		width: 100%;
	}

	input {
		background-color: white;
	}

	input[type=checkbox] {
		margin-top: 1em;
		min-width: unset;
		width: unset;
	}

	label {
		user-select: none;
	}

	select {
		background-color: buttonface;
	}

	input::placeholder {
		color: black;
		opacity: 0.75;
	}

	span {
		display: flex;
		justify-content: space-around;
	}

	a {
		margin-left: 0.25em;
		margin-right: 0.25em;
		white-space: nowrap;
	}
</style>
