<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
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

	function onSubmit() {
		callback($message);
		message.set('');
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
	<Section name='Radio'>
		<table>
			{#each $chats as {name, team, message}}
				<tr>
					<td class='name'>{team ? `[${team}] ${name}` : name}</td>
					<td class='message'>{message}</td>
				</tr>
			{/each}
		</table>
		{#if auto($message)}
			<p><b>Automated help: {auto($message)}</b></p>
		{/if}
		<form on:submit|preventDefault={onSubmit}>
			<input type='text' name='message' placeholder='Message' autocomplete='off' minLength={1} maxLength={100} value={$message} on:input={onInput} bind:this={input}/>
		</form>
	</Section>
</div>

<style>
	div {
		bottom: 0;
		background-color: #00000040;
		margin: 10px;
		max-width: 25%;
		padding: 10px;
		position: absolute;
		right: 0;
	}

	h2 {
		margin-bottom: 10px;
		margin-top: 0px;
	}

	table {
		width: 100%;
	}

	td {
		text-align: left;
	}

	td.name {
		font-weight: bold;
		white-space: nowrap;
		width: 1%;
	}

	td.message {
		word-break: break-all;
	}
</style>
