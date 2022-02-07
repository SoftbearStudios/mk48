<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import {writable} from 'svelte/store';
	// Store message in module context to persist it between player deaths.
	const message = writable('');
</script>

<script>
	import Section from '../component/Section.svelte';
	import {chatShown, cinematic} from '../util/settings.js';
	import t from '../util/translation.js';
	import {showContextMenu} from './ContextMenu.svelte';

	export let onMutePlayer;
	export let onReportAbuse;
	export let onSendChat;
	export let state;

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
		const team = event.shiftKey && state.teamName != null;
		input && input.blur && input.blur();
		if ($message == '') {
			return;
		}
		onSendChat($message, team);
		$message = '';
	}

	function auto(msg) {
		if (!msg) {
			return;
		}

		if (msg.includes('/invite')) {
			return 'Invitation links cannot currently be accepted by players that are already in game. They must send a join request instead.';
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

<div id="chat" class:cinematic={$cinematic}>
	<Section name={$t('panel.chat.label')} headerAlign='right' bind:open={$chatShown}>
		{#each state.chats as {name, playerId, team, whisper, message}}
			<p class='message' class:whisper>
				<span
					class='name'
					class:official={playerId == undefined}
					on:click={() => populateReply(name)}
					on:contextmenu={typeof playerId === 'number' && playerId !== state.playerId ? (event => showContextMenu(event, {[`Report ${name}`]: () => onReportAbuse(playerId), [`Mute ${name}`]: () => onMutePlayer(playerId, true)})) : null}
				>{team ? `[${team}] ${name}` : name}</span>&nbsp;{message}
			</p>
		{/each}
		{#if auto($message)}
			<p><b>Automated help: {auto($message)}</b></p>
		{/if}
		<input type='text' name='message' title={$t(`panel.chat.action.send.hint${state.teamName ? 'Team' : ''}`)} placeholder={$t('panel.chat.action.send.label')} autocomplete='off' minLength={1} maxLength={128} value={$message} on:input={onInput} on:keydown={onKeyDown} bind:this={input}/>
	</Section>
</div>

<style>
	#chat {
		bottom: 0;
		max-width: 25%;
		padding-bottom: 1rem;
		padding-right: 1rem;
		position: absolute;
		right: 0;
	}

	div {
		max-width: 25%;
	}

	div.cinematic:not(:hover) {
		opacity: 0;
	}

	input {
		width: 100%;
	}

	.official {
		color: #fffd2a;
		text-shadow: 0px 0px 3px #381616;
	}

	p {
		color: white;
	}

	p.message {
		margin-bottom: 0.25em;
		margin-top: 0.25em;
		overflow-wrap: anywhere;
		text-overflow: ellipsis;
		word-break: normal;
	}

	p.whisper {
		filter: brightness(0.7);
	}

	span.name {
		cursor: pointer;
		font-weight: bold;
		white-space: nowrap;
	}
</style>
