<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';
	import storage from '../util/storage.js';
	import {getInvite} from './teams.js';
	import {deathReason} from './socket.js';
	import {onMount} from 'svelte';
	import {fade} from 'svelte/transition';
	import t, {setLanguage, translateAs} from './translation.js';
	import strings from '../data/strings.json';
	import {browser} from '$app/env';
	import Link, {outboundEnabled} from './Link.svelte';
	import {summarizeType} from './Ship.svelte';

	export let callback;
	export let connectionLost = false;

	function getSpawnable() {
		const list = [];
		for (const entityType of Object.keys(entityData)) {
			const data = entityData[entityType];
			// JS doesn't know if the auth is correct, so give user the benefit
			// of the doubt (server will enforce)
			if (data.kind === 'boat' && ((data.level === 1 && !data.npc) || storage.auth)) {
				list.push(entityType);
			}
		}
		return list;
	}

	function getRandomSpawnable() {
		const spawnable = getSpawnable();
		return spawnable[Math.floor(Math.random() * spawnable.length)];
	}

	let name = storage.name || '';
	let type = storage.type || getRandomSpawnable();
	let invite = undefined;
	let paused = false;

	onMount(() => {
		invite = parseInviteCode(getInvite());

		if (window.parent && window.parent.postMessage) {
			try {
				window.parent.postMessage('splash', '*');
			} catch (err) {
				console.warn(err);
			}
		}
	});

	// Message originates from iframe parent
	function handleMessage(event) {
		console.log(`game received message: ${event.data}`);
		switch (event.data) {
			case 'pause':
				paused = true;
				break;
			case 'unpause':
				paused = false;
				break;
			case 'disableOutbound':
				outboundEnabled.set(false);
				break;
		}
	}

	function parseInviteCode(invite) {
		try {
			const segments = invite.split('/');
			return segments[1];
		} catch (err) {
			return undefined;
		}
	}

	const minNameLength = 3;
	const maxNameLength = 12;

	function handleSubmit() {
		if (name) {
			storage.name = name;
		}
		storage.type = type;
		callback({
			name: name || 'Guest',
			type,
			auth: storage.auth,
			invite,
			new: storage.join == undefined
		});

		if (storage.join == undefined) {
			storage.join = Date.now();
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

	// Link target
	$: target = $outboundEnabled ? '_blank' : null;
</script>

<div class='splash' in:fade="{{delay: 2000, duration: 500}}">
	<h2>{$t('panel.splash.label')}</h2>
	{#if connectionLost}
		<p>{$t('panel.splash.connectionLost')}</p>
	{:else if $deathReason}
		<p>{fmtDeathReason($t, $deathReason)}</p>
	{/if}
	<!--<small>Server maintainance in progress</small>-->
	<form on:submit|preventDefault|stopPropagation={handleSubmit}>
		<input type='text' name='name' placeholder='Nickname' autocomplete='off' minlength={minNameLength} maxlength={maxNameLength} bind:value={name}/>
		<select bind:value={type}>
			{#each getSpawnable() as type}
				<option value={type}>{entityData[type].label}</option>
			{/each}
		</select>
		<button disabled={!type || (name && (name.length < minNameLength || name.length > maxNameLength)) || paused} on:click={handleSubmit}>{$t('panel.splash.action.spawn.label')}</button>
		{#if invite}
			<small>{$t('panel.splash.invitePrefix')} {invite}</small>
		{/if}
	</form>
	<span>
		<a href='/help' {target}>{$t('panel.splash.action.help.label')}</a>
		<a href='/about' {target}>{$t('panel.splash.action.about.label')}</a>
		<a href='/privacy' {target}>{$t('panel.splash.action.privacy.label')}</a>
		<a href='/terms' {target}>{$t('panel.splash.action.terms.label')}</a>
	</span>
</div>

<div class='translation' in:fade="{{delay: 2000, duration: 500}}">
	<h3>Language</h3>
	<select value={storage.language} on:change={e => setLanguage(e.target.value)}>
		{#each Object.keys(strings) as lang}
			{#if Object.keys(strings[lang]).length > 0}
				<option value={lang}>{translateAs(lang, 'label')}</option>
			{/if}
		{/each}
	</select>
</div>

<svelte:window on:message={handleMessage} on:hashchange={() => invite = parseInviteCode(getInvite())}/>

<style>
	div {
		background-color: white;
		border-radius: 0.5em;
		box-shadow: 0em 0.2em 0 #cccccc;
		color: black;
		height: min-content;
		margin: auto;
		padding: 1em;
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
		right: 10px;
		bottom: 10px;
	}

	h2, h3 {
		margin: 0;
	}

	form {
		padding-bottom: 10px;
	}

	input, select {
		border: 1px solid gray;
		color: black;
		cursor: pointer;
		margin-top: 5px;
		min-width: 200px;
		outline: 0px;
		padding: 8px;
		width: 100%;
	}

	input {
		background-color: white;
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
