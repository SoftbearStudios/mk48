<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import {teamMembers} from './socket.js';
	import t from './translation.js';
	import {writable} from 'svelte/store';
	// Store message in module context to persist it between
	// player deaths (See #22)
	let message = writable('');
</script>

<script>
	import {chats} from '../lib/socket.js';
	import Section from './Section.svelte';

	export let callback;

	let input; // ref

	function onInput(event) {
		message.set(event.target.value);
	}

	function populateReply(name) {
		if (!$message || ($message.trim().length === 0) || $message.startsWith('@')) {
			$message = `@${name} `;
		}
	}

	export function blur() {
		input && input.blur && input.blur();
	}

	export function focus() {
		input && input.focus && input.focus();
	}

	function onKeyDown(event) {
		// Enter key
		if (event.keyCode !== 13) {
			return;
		}
		const team = event.shiftKey;
		if (team && !($teamMembers)) {
			return;
		}
		callback({message: $message, team});
		$message = '';
		input && input.blur && input.blur();
	}

	function auto(msg) {
		if (!msg) {
			return;
		}

		if (msg.includes('how')) {
			if (msg.includes('move')) {
				return 'If you are asking how you move, you click and hold (or right click) outside the inner ring of your ship to set your speed and direction (or use WASD)';
			}

			if (msg.includes('play')) {
				return 'The controls are click and hold (or WASD) to move, click (or Space) to shoot';
			}

			if (msg.includes('shoot') || msg.includes('use weapons') || msg.includes('fire')) {
				return 'First, select an available weapon. Then, click in the direction to fire. If you hold the click for too long, you won\'t shoot.';
			}
		}

		return;
	}
</script>

<div>
	<Section name={$t('panel.chat.label')} headerAlign='right'>
		{#each $chats as {name, team, teamOnly, message}}
			<p class='message' class:teamOnly>
				<span class='name' on:click={() => populateReply(name)}>{team ? `[${team}] ${name}` : name}</span>&nbsp;{message}
			</p>
		{/each}
		{#if auto($message)}
			<p><b>Automated help: {auto($message)}</b></p>
		{/if}
		<input type='text' name='message' title={$t(`panel.chat.action.send.hint${$teamMembers ? 'Team' : ''}`)} placeholder={$t('panel.chat.action.send.label')} autocomplete='off' minLength={1} maxLength={128} value={$message} on:input={onInput} on:keydown={onKeyDown} bind:this={input}/>
	</Section>
</div>

<style>
	div {
		max-width: 25%;
	}

	h2 {
		margin-bottom: 10px;
		margin-top: 0px;
	}

	table {
		width: 100%;
	}

	p.message {
		text-overflow: ellipsis;
		overflow-wrap: anywhere;
		word-break: normal;
		margin-top: 0.25em;
		margin-bottom: 0.25em;
	}

	p.teamOnly {
		filter: brightness(0.7);
	}

	span.name {
		cursor: pointer;
		font-weight: bold;
		white-space: nowrap;
	}

	input {
		width: 100%;
	}
</style>
