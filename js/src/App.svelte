<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context="module">
	import storage from './util/storage.js';
	import {get, writable} from 'svelte/store';

	let state = writable(null);

	export function setProps(value) {
		state.set(value);
	}

	// "real" host (that is to say, if the window.location.host redirected to some other host).
	let realHost = null;
	let realEncryption = null;

	export function getRealHost() {
		return realHost;
	}

	export function getRealEncryption() {
		return realEncryption;
	}

	// Find and cache the above.
	async function findRealHost() {
		try {
			// This url:
			// 1) was preloaded in HTML
			// 2) is needed later, so this has no additional network overhead
			// 3) is small enough to not matter if 1-2 are false
			// 4) Is simply used to trace any redirects that occur
			const response = await fetch("/favicon.png");
			const url = new URL(response.url);
			realHost = url.host;
			realEncryption = url.protocol != 'http:';
		} catch (err) {
			console.warn(err);
		}
	}
</script>

<script>
	import ContextMenu from './lib/ContextMenu.svelte';
	import Router from 'svelte-spa-router';
	import Help from './page/Help.svelte';
	import About from './page/About.svelte';
	import Privacy from './page/Privacy.svelte';
	import Terms from './page/Terms.svelte';
	import Settings from './page/Settings.svelte';
	import Ships from './page/Ships.svelte';
	import Levels from './page/Levels.svelte';
	import Changelog from './page/Changelog.svelte';
	import Chat from './lib/Chat.svelte';
	import Instructions from './lib/Instructions.svelte';
	import Leaderboard from './lib/Leaderboard.svelte';
	import Ship from './lib/Ship.svelte';
	import Sidebar from './lib/Sidebar.svelte';
	import SplashScreen from './lib/SplashScreen.svelte';
	import Status from './lib/Status.svelte';
	import Teams from './lib/Teams.svelte';
	import Hint from './lib/Hint.svelte';
	import t from './lib/translation.js';
	import Upgrades, {canUpgrade} from './lib/Upgrades.svelte';
	import wasm from '../../client/Cargo.toml';
	import {getMouseButton} from './util/compatibility.js';
	import {mapRanges} from './util/math.js';
	import {onMount} from 'svelte';
	import {antialias, cinematic, renderTerrainTextures, volume, waveQuality, resolution} from './util/settings.js';
	import {outboundEnabled} from './lib/Link.svelte';

	let canvas, chatRef, shipRef, client, innerWidth, innerHeight, animationFrameRequest;
	let active, altitudeTarget, armamentSelection;
	let instructBasics = true;
	let instructZoom = true;
	const keyboard = {};

	$: client && typeof active === 'boolean' && client.event({"Active": active});
	$: client && typeof altitudeTarget === 'number' && client.event({"AltitudeTarget": altitudeTarget});
	$: client && typeof armamentSelection === 'string' && client.event({"Armament": armamentSelection.split('/')});
	//$: client && client.handleVolume($volume);
	$: client && client.event({"Cinematic": $cinematic});

	onMount(async () => {
		await findRealHost();

		client = await wasm();

		//client.run(host, encrypted, settings, storage.arenaId, storage.sessionId, invitationId);
		animationFrameRequest = requestAnimationFrame(onAnimationFrame);

		// Make client accessible (for debugging only).
		window.rust = client;
	});

	function onAnimationFrame(timestamp) {
		client && client.frame(timestamp / 1000.0);
		animationFrameRequest = requestAnimationFrame(onAnimationFrame);
	};

	function safeNumber(num) {
		return typeof num === 'number' && isFinite(num);
	}

	function onSpawn(alias, entityType) {
		client && client.event({"Spawn": {alias, entityType}});
	}

	function onMouseButton(event) {
		event.stopPropagation();

		chatRef && chatRef.blur && chatRef.blur();

		const button = getMouseButton(event);
		const down = {mousedown: true, mouseup: false}[event.type];

		if (typeof down == 'boolean' && [0, 2].includes(button)) {
			instructBasics = false;
		}

		client && client.mouse(event);
	}

	// Equals distance between touches if a pinch operation is in progress.
	let pinch = null;

	// Also handles touch moves.
	function onMouseMove(event) {
		client && client.mouse(event);
	}

	let touch = false;

	function onTouch(event) {
		if (['touchstart', 'touchend'].includes(event.type)) {
			touch = true;
		}

		client && client.touch(event);
	}

	function onKey(event) {
		client && client.keyboard(event);

		const {ctrlKey, keyCode, preventDefault, shiftKey, target, type} = event;

		const down = {keydown: true, keyup: false}[type];

		if (down && target && (target instanceof HTMLInputElement)) {
			return;
		}

		if (down !== undefined) {
			// Key bindings.
			const keys = {};

			if (down && ctrlKey) {
				// No-op.
			} else {
				if (chatRef && chatRef.focus) {
					// enter
					keys[13] = chatRef.focus.bind(chatRef);
				}

				// Last 3 checks to prevent https://github.com/SoftbearStudios/mk48/issues/26
				if (shipRef && shipRef.toggleActive && shipRef.toggleAltitudeTarget && shipRef.incrementSelection && shipRef.setSelectionIndex) {
					// tab
					Object.assign(keys, {
						9: () => shipRef.incrementSelection(),   // tab
						90: () => shipRef.toggleActive(),        // z
						82: () => shipRef.toggleAltitudeTarget() // r
					});;

					// numbers
					for (let i = 0; i < 9; i++) {
						keys[49 + i] = () => shipRef.setSelectionIndex(i);
					}
				}
			}

			const key = keys[keyCode];

			if (key) {
				if (typeof key === 'function') {
					if (down) {
						key();
					}
				} else if (!(down && keyboard[key])) {
					// Don't reset date if already down.
					keyboard[key] = down ? Date.now() : false;
				}

				event.preventDefault();
				event.stopPropagation();
			}
		}
	}

	function onWheel(event) {
		client && client.wheel(event);

		instructZoom = false;
	};

	function onUpgrade(type) {
		instructBasics = false;

		client && client.event({"Upgrade": type});
	}

	function onSendChat(message, team) {
		client && client.handleSendChat(message, team);
	}

	function onReportPlayer(playerId) {
		client && client.handleReportPlayer(playerId);
	}

	function onMutePlayer(playerId, mute) {
		client && client.handleMutePlayer(playerId, mute);
	}

	function onCreateTeam(name) {
		client && client.handleCreateTeam(name);
	}

	function onRequestJoinTeam(teamId) {
		client && client.handleRequestJoinTeam(teamId);
	}

	function onAcceptJoinTeam(playerId) {
		client && client.handleAcceptJoinTeam(playerId);
	}

	function onRejectJoinTeam(playerId) {
		client && client.handleRejectJoinTeam(playerId);
	}

	function onKickFromTeam(playerId) {
		client && client.handleKickFromTeam(playerId);
	}

	function onLeaveTeam() {
		client && client.handleLeaveTeam();
	}

	async function onCopyInvitationLink() {
		let invitationId = get(state).invitationId;
		if (invitationId) {
			await navigator.clipboard.writeText(`${location.origin}/#/invite/${invitationId}`);
		}
	}

	function onMouseFocus(event) {
		client && client.mouseFocus(event);
	}

	function onMouseLeave(event) {
		client && client.mouse(event);
	}

	function onChangeWindowFocused(focused) {
		client && client.keyboardFocus(event);
	}

	// Originates from iframe parent.
	// Also handled by lib/SplashScreen.svelte
	function onMessage(event) {
		console.log(`game received message: ${event.data}`);
		switch (event.data) {
			case 'mute':
				volume.set(0);
				break;
			case 'unmute':
				volume.setDefault();
				break;
			case 'disableOutbound':
				outboundEnabled.set(false);
				break;
		}
	}

	let traces = 0;
	async function onError(event) {
		if (traces >= 10) {
			return;
		}
		traces++;
		const message = typeof event === 'string' ? event : (event.error ? `${event.error.message}: ${event.error.stack}` : (event.message || JSON.stringify(event.reason)));
		const response = await fetch(`/client/`, {
			method: 'POST',
			body: JSON.stringify({request: {Trace: {message}}, params: {arena_id: null, session_id: null, newbie: false}}),
			headers: {
				'Content-Type': 'application/json'
			}
		});
	}

	function processWindowDimension(dim, res) {
		return Math.floor(dim * res * (window.devicePixelRatio || 1.0));
	}
</script>

<canvas
	id='canvas'
	bind:this={canvas}
	width={processWindowDimension(innerWidth, $resolution)}
	height={processWindowDimension(innerHeight, $resolution)}
	on:mousedown|preventDefault={onMouseButton}
	on:mouseup|preventDefault={onMouseButton}
	on:mousemove|preventDefault={onMouseMove}
	on:touchstart={onTouch}
	on:touchend={onTouch}
	on:touchmove={onTouch}
	on:blur={onMouseFocus}
	on:mouseleave={onMouseLeave}
	on:wheel|preventDefault={onWheel}
	on:contextmenu|preventDefault
></canvas>

<div class='top bar'>
	{#if client && $state && $state.status.alive}
		<Teams state={$state} {onCreateTeam} {onRequestJoinTeam} {onAcceptJoinTeam} {onRejectJoinTeam} {onKickFromTeam} {onLeaveTeam}/>
		{#if canUpgrade($state.status.alive.type, $state.score)}
			<Upgrades
				type={$state.status.alive.type}
				{onUpgrade}
			/>
		{:else}
			<Instructions {touch} {instructBasics} {instructZoom}/>
		{/if}
	{:else}
		<!-- Render this div even without contents, as it causes the flex
		box to shift the other contents to the right side -->
		<div>
			<Leaderboard label={$t('panel.leaderboard.type.single/all')} leaderboard={$state && $state.leaderboards && $state.leaderboards['AllTime'] ? $state.leaderboards['AllTime'] : []} headerAlign='left'/>
			<br/>
			<Leaderboard label={$t('panel.leaderboard.type.single/week')} open={false} leaderboard={$state && $state.leaderboards && $state.leaderboards['Weekly'] ? $state.leaderboards['Weekly'] : []} headerAlign='left'/>
			<br/>
			<Leaderboard label={$t('panel.leaderboard.type.single/day')} open={false} leaderboard={$state && $state.leaderboards && $state.leaderboards['Daily'] ? $state.leaderboards['Daily'] : []} headerAlign='left'/>
		</div>
	{/if}
	{#if $state && $state.liveboard}
		<Leaderboard leaderboard={$state.liveboard} headerAlign='right' footer={$state.playerCount ? $t('panel.online.label').replace('{players}', $state.playerCount) : null}/>
	{:else}
		<div><!--placeholder--></div>
	{/if}
</div>

{#if client}
	{#if $state && $state.status}
		{#if $state.status.alive}
			<div class='bottom bar'>
				<Ship bind:this={shipRef} state={$state} bind:active bind:altitudeTarget bind:selection={armamentSelection}/>
				<Status state={$state}/>
				<Chat bind:this={chatRef} state={$state} {onSendChat} {onMutePlayer} {onReportPlayer}/>
			</div>
			{#if !$cinematic}
				<Hint type={$state.status.alive.type}/>
			{/if}
		{:else if $state.status.spawning}
			<SplashScreen state={$state} {onSpawn}/>
		{/if}
	{/if}
	<Sidebar onZoom={client.zoom} {onCopyInvitationLink}/>
{/if}
<ContextMenu/>
<Router routes={{'/help': Help, '/about': About, '/settings': Settings, '/privacy': Privacy, '/terms': Terms, '/ships': Ships, '/levels': Levels, '/changelog': Changelog}}></Router>

<svelte:head>
	<title>mk48.io</title>
</svelte:head>
<svelte:window
	bind:innerWidth bind:innerHeight
	on:keydown={onKey} on:keyup={onKey}
	on:blur={() => onChangeWindowFocused(false)}
	on:focus={() => onChangeWindowFocused(true)}
	on:message={onMessage}
	on:error={onError} on:unhandledrejection={onError}
/>

<style>
	:global(*) {
		font-family: sans-serif;
	}

	:global(body) {
		margin: 0;
		padding: 0;
		touch-action: pan-y;
	}

	canvas {
		position: absolute;
		width: 100%;
		height: 100%;
	}

	div.bar {
		position: absolute;
		left: 0;
		right: 0;
		height: min-content;
		pointer-events: none;
		display: flex;
		justify-content: space-between;
	}

	div.bar.top {
		top: 0;
	}

	div.bar.bottom {
		bottom: 0;
		align-items: flex-end;
	}

	div.bar > :global(div) {
		height: min-content;
		pointer-events: all;
		margin: 1em;
	}

	:global(input), :global(select) {
		border-radius: 0.25em;
		box-sizing: border-box;
		cursor: pointer;
		font-size: 1em;
		font-weight: bold;
		outline: 0;
		padding: 0.7em;
		pointer-events: all;
		white-space: nowrap;
		margin-top: 0.25em;

		background-color: #00000025;
		border: 0;
		color: white;
	}

	:global(input::placeholder) {
		opacity: 0.75;

		color: white;
	}

	:global(button) {
		background-color: #61b365;
		border: 1px solid #61b365;
		border-radius: 0.25em;
		box-sizing: border-box;
		color: white;
		cursor: pointer;
		font-size: 1em;
		margin-top: 0.5em;
		padding: 0.5em 0.6em;
		text-decoration: none;
		white-space: nowrap;
		width: 100%;
	}

	:global(button:disabled) {
		filter: opacity(0.6);
	}

	:global(button:hover:not(:disabled)) {
		filter: brightness(0.95);
	}

	:global(button:active:not(:disabled)) {
		filter: brightness(0.9);
	}

	:global(html) {
		font-size: 1.4vmin;
		font-size: calc(5px + 0.9vmin);
	}
</style>
