<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context="module">
	import storage from './util/storage.js';
	import {get, writable} from 'svelte/store';

	let state = writable(null);
	let leaderboard = writable(null);
	let globalLeaderboard = writable(null);

	export function setSessionId(arenaId, sessionId) {
		storage.arenaId = arenaId;
		storage.sessionId = sessionId;
	}

	export function setState(value) {
		state.set(value);
	}
</script>

<script>
	import ContextMenu from './lib/ContextMenu.svelte';
	import Router from 'svelte-spa-router';
	import Help from './page/Help.svelte';
	import About from './page/About.svelte';
	import Privacy from './page/Privacy.svelte';
	import Terms from './page/Terms.svelte';
	import Ships from './page/Ships.svelte';
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
	import {volume} from './util/settings.js';
	import {outboundEnabled} from './lib/Link.svelte';

	let canvas, chatRef, shipRef, client, innerWidth, innerHeight, animationFrameRequest;
	let active, altitudeTarget, armamentSelection;
	let instructBasics = true;
	let instructZoom = true;
	const keyboard = {};

	$: client && typeof active === 'boolean' && client.handleActive(active);
	$: client && typeof altitudeTarget === 'number' && client.handleAltitudeTarget(altitudeTarget);
	$: client && typeof armamentSelection === 'string' && client.handleArmamentSelection(armamentSelection);
	$: client && client.handleVolume($volume);

	onMount(async () => {
		client = await wasm();

		const hash = window.location.hash;
		let invitationId = null;

		if (hash.includes("/invite")) {
			invitationId = hash.split("/").pop();
		}

		client.run(storage.arenaId, storage.sessionId, invitationId);
		animationFrameRequest = requestAnimationFrame(onAnimationFrame);

		// Make client accessible (for debugging only).
		window.rust = client;
	});

	function onAnimationFrame(timestamp) {
		// Joystick input.
		let forwardBackward = 0;
		let leftRight = 0;

		if (keyboard.forward) {
			forwardBackward += 1;
		}
		if (keyboard.backward) {
			forwardBackward -= 1;
		}
		if (keyboard.left) {
			leftRight += mapRanges(Date.now() - keyboard.left, 0, 1000, 0.25, 1, true);
		}
		if (keyboard.right) {
			leftRight -= mapRanges(Date.now() - keyboard.right, 0, 1000, 0.25, 1, true);
		}
		if (!keyboard.stop && forwardBackward === 0 && leftRight === 0) {
			client.handleJoystickRelease();
		} else {
			instructBasics = false;
			client.handleJoystick(leftRight, forwardBackward, typeof keyboard.stop === 'number');
		}

		client && client.frame(timestamp / 1000.0);
		animationFrameRequest = requestAnimationFrame(onAnimationFrame);
	};

	function onSpawn(name, type) {
		client && client.handleSpawn(name, type);
	}

	function onMouseButton(event) {
		event.stopPropagation();

		chatRef && chatRef.blur && chatRef.blur();

		const button = getMouseButton(event);

		const down = {mousedown: true, mouseup: false}[event.type];

		if (typeof down == 'boolean' && [0, 2].includes(button)) {
			instructBasics = false;
			client && client.handleMouseButton(button, down);
		}
	}

	// Equals distance between touches if a pinch operation is in progress.
	let pinch = null;

	// Also handles touch moves.
	function onMouseMove(event) {
		let pos = event;
		if (event.touches && event.touches.length > 0) {
			// Each touch has its own pageX and pageY, just like event.
			pos = event.touches[0];

			if (event.touches.length === 2) {
				const currentDistance = Math.hypot(event.touches[0].pageX - event.touches[1].pageX, event.touches[0].pageY - event.touches[1].pageY);
				if (pinch) {
					// Pinch is interpreted as zoom.
					instructZoom = false;

					client && client.handleWheel(0.15 * (pinch - currentDistance));
					pinch = currentDistance;
				} else {
					pinch = currentDistance;
				}

				// Don't issue a mouse move.
				return;
			} else {
				pinch = null;
			}
		} else {
			pinch = null;
		}

		if (typeof pos.pageX === 'number') {
			const rect = canvas.getBoundingClientRect();
			const aspect = rect.height / rect.width;
			const x = mapRanges(pos.pageX, rect.x, rect.x + rect.width, -1, 1);
			const y = mapRanges(pos.pageY, rect.y, rect.y + rect.height, aspect, -aspect);
			client && client.handleMouseMove(x, y);
		}
	}

	let touch = false;

	function onTouch(event) {
		event.preventDefault();

		const button = getMouseButton(event);

		if (['touchstart', 'touchend'].includes(event.type)) {
			touch = true;

			// Simulate left button.
			client && client.handleMouseButton(0, event.type === 'touchstart');
		}

		onMouseMove(event);
	}

	function onKey(event) {
		const {ctrlKey, keyCode, preventDefault, shiftKey, target, type} = event;

		const down = {keydown: true, keyup: false}[type];

		if (down && target && (target instanceof HTMLInputElement)) {
			return;
		}

		if (down !== undefined) {
			// Key bindings.
			const keys = {};

			if (down && ctrlKey) {
				Object.assign(keys, {
					// Zoom in and out (see #57).
					187: () => client.handleWheel(-1.0), // +
					189: () => client.handleWheel(1.0)   // -
				});
			} else {
				Object.assign(keys, {
					32: 'shoot', // space
					67: 'pay',   // c (coin)
					69: 'shoot', // e
					88: 'stop',  // x

					// WASD
					65: 'left',     // a
					87: 'forward',  // w
					68: 'right',    // d
					83: 'backward', // s

					// arrows
					37: 'left',     // left arrow
					38: 'forward',  // up arrow
					39: 'right',    // right arrow
					40: 'backward', // backward arrow
				});

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
				} else if (key === 'pay') {
					client && client.handlePay(down);
				} else if (key === 'shoot') {
					client && client.handleShoot(down);
				} else if (!(down && keyboard[key])) {
					// Don't reset date if already down.
					keyboard[key] = down ? Date.now() : false;

					// A common reason for stopping is that forward/backward is stuck.
					if (key === 'stop' && down) {
						keyboard.forward = false;
						keyboard.backward = false;
					}
				}

				event.preventDefault();
				event.stopPropagation();
			}
		}
	}

	function onWheel(event) {
		instructZoom = false;

		const delta = event.deltaY * 0.05;
		client && client.handleWheel(delta);
	};

	function onUpgrade(type) {
		instructBasics = false;

		client && client.handleUpgrade(type);
	}

	function onSendChat(message, team) {
		client && client.handleSendChat(message, team);
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
		const message = typeof event === 'string' ? event : (event.error ? `${event.error.message}: ${event.error.stack}` : (event.message || event.reason));
		const response = await fetch(`/client/`, {
			method: 'POST',
			body: JSON.stringify({request: {Trace: {message}}, params: {arena_id: null, session_id: null, newbie: false}}),
			headers: {
				'Content-Type': 'application/json'
			}
		});
	}

	function processWindowDimension(dim) {
		return Math.floor(dim * (window.devicePixelRatio || 1.0));
	}
</script>

<canvas
	id='canvas'
	bind:this={canvas}
	width={processWindowDimension(innerWidth)}
	height={processWindowDimension(innerHeight)}
	on:mousedown|preventDefault={onMouseButton}
	on:mouseup|preventDefault={onMouseButton}
	on:mousemove|preventDefault={onMouseMove}
	on:touchstart={onTouch}
	on:touchend={onTouch}
	on:touchmove={onMouseMove}
	on:wheel|preventDefault={onWheel}
	on:contextmenu|preventDefault
></canvas>

{#if client && $state}
	<div class='top bar'>
		{#if $state.status.alive}
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
				{#if $state.leaderboards}
					{#if $state.leaderboards['AllTime']}
						<Leaderboard label={$t('panel.leaderboard.type.single/all')} leaderboard={$state.leaderboards['AllTime']} headerAlign='left'/>
						<br/>
					{/if}
					{#if $state.leaderboards['Weekly']}
						<Leaderboard label={$t('panel.leaderboard.type.single/week')} open={false} leaderboard={$state.leaderboards['Weekly']} headerAlign='left'/>
						<br/>
					{/if}
					{#if $state.leaderboards['Daily']}
						<Leaderboard label={$t('panel.leaderboard.type.single/day')} open={false} leaderboard={$state.leaderboards['Daily']} headerAlign='left'/>
					{/if}
				{/if}
			</div>
		{/if}
		{#if $state.liveboard}
			<Leaderboard leaderboard={$state.liveboard} headerAlign='right' footer={$state.playerCount ? `${$state.playerCount} online` : null}/>
		{:else}
			<div><!--placeholder--></div>
		{/if}
	</div>
	{#if $state.status.alive}
		<div class='bottom bar'>
			<Ship bind:this={shipRef} state={$state} bind:active bind:altitudeTarget bind:selection={armamentSelection}/>
			<Status state={$state}/>
			<Chat bind:this={chatRef} state={$state} {onSendChat} {onMutePlayer}/>
		</div>
		<Hint type={$state.status.alive.type}/>
	{:else if $state.status.spawning}
		<SplashScreen state={$state} {onSpawn}/>
	{/if}
	<Sidebar onZoom={client.handleWheel} {onCopyInvitationLink}/>
	<ContextMenu/>
{/if}
<Router routes={{'/help': Help, '/about': About, '/privacy': Privacy, '/terms': Terms, '/ships': Ships, '/changelog': Changelog}}></Router>

<svelte:head>
	<title>mk48.io</title>
</svelte:head>
<svelte:window
	bind:innerWidth bind:innerHeight
	on:keydown={onKey} on:keyup={onKey}
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
		background-color: #2980b9;
		border: 1px solid #2980b9;
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
