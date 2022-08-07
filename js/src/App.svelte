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

	function parseInvitationId() {
		try {
			const s = window.location.hash.split('/');
			const last = s[s.length - 1];
			const invitationId = parseInt(last);
			if (invitationId > 0) {
				return invitationId;
			} else {
				return null;
			}
		} catch (err) {
			return null;
		}
	}

	// "real" host (that is to say, if the window.location.host redirected to some other host).
	let realHost = null;
	let realEncryption = null;
	let idealServerId = null;

	export function getRealHost() {
		return realHost;
	}

	export function getRealEncryption() {
		return realEncryption;
	}

	export function getIdealServerId() {
		return idealServerId;
	}

	// Find and cache the above.
	async function findRealHost() {
		try {
			// Serves two purposes: Tracing redirects, and getting ideal server id.
			const params = {};
			let invitationId = parseInvitationId();
			if (invitationId != null) {
				params.invitation_id = invitationId;
			}
			if (sessionStorage && sessionStorage.serverId && !isNaN(sessionStorage.serverId)) {
				params.server_id = sessionStorage.serverId;
			}
			const response = await fetch(`/system/?${new URLSearchParams(params).toString()}`);
			const url = new URL(response.url);
			realHost = url.host;
			realEncryption = url.protocol != 'http:';
			const body = await response.json();
			idealServerId = body.server_id;
		} catch (err) {
			console.warn(err);
		}
	}
</script>

<script>
	import AboutDialog from './dialog/AboutDialog.svelte';
	import Changelog from './dialog/Changelog.svelte';
	import Chat from './overlay/Chat.svelte';
	import ContextMenu from './overlay/ContextMenu.svelte';
	import HelpDialog from './dialog/HelpDialog.svelte';
	import HelpLinks from './overlay/HelpLinks.svelte';
	import Hint from './overlay/Hint.svelte';
	import Instructions from './overlay/Instructions.svelte';
	import Leaderboard from './overlay/Leaderboard.svelte';
	import LevelsDialog from './dialog/LevelsDialog.svelte';
	import PrivacyDialog from './dialog/PrivacyDialog.svelte';
	import ProgressSpinner from './overlay/ProgressSpinner.svelte';
	import LanguageMenu from './component/LanguageMenu.svelte';
	import XButton from './component/XButton.svelte';
	import RespawnMenu from './overlay/RespawnMenu.svelte';
	import Router from 'svelte-spa-router';
	import {wrap} from 'svelte-spa-router/wrap'
	import SettingsDialog from './dialog/SettingsDialog.svelte';
	import ShipControls from './overlay/ShipControls.svelte';
	import ShipsDialog from './dialog/ShipsDialog.svelte';
	import ShipStatus from './overlay/ShipStatus.svelte';
	import Sidebar from './overlay/Sidebar.svelte';
	import SpawnOverlay from './overlay/SpawnOverlay.svelte';
	import TeamsOverlay from './overlay/TeamsOverlay.svelte';
	import TermsDialog from './dialog/TermsDialog.svelte';
	import WarningPanel from './overlay/WarningPanel.svelte';
	import t from './util/translation.js';
	import UpgradeMenu from './overlay/UpgradeMenu.svelte';
	import wasm from '../../client/Cargo.toml';
	import {canUpgrade} from './util/warship.js';
	import {getMouseButton} from './util/compatibility.js';
	import {mapRanges} from './util/math.js';
	import {onMount} from 'svelte';
	import {antialias, cinematic, volume, waveQuality, resolution, loadRustSettings} from './util/settings.js';
	import {outboundEnabled} from './component/Link.svelte';

	let chatRef, shipRef, client, innerWidth, innerHeight, animationFrameRequest;
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

		animationFrameRequest = requestAnimationFrame(onAnimationFrame);

		// Make client accessible (for debugging only).
		window.rust = client;
		loadRustSettings();
	});

	function onAnimationFrame(timestamp) {
		client && client.frame(timestamp / 1000.0);
		animationFrameRequest = requestAnimationFrame(onAnimationFrame);
	}

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

	function onReportAbuse(playerId) {
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

	function onChangeWindowFocused(event) {
		client && client.keyboardFocus(event);
	}

	// Originates from iframe parent.
	// Also handled by overlay/SpawnOverlay.svelte
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
		client && client.handleTrace(message);
	}

	function processWindowDimension(dim, res) {
		return Math.floor(dim * res * (window.devicePixelRatio || 1.0));
	}
</script>

<canvas
	id='canvas'
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

{#if !(client && $state && $state.status)}
	<ProgressSpinner />
{:else if $state.status == 'offline'}
	<WarningPanel message={$t('panel.splash.connectionLost')} />
{:else if $state.status == 'spawning'}
	<SpawnOverlay {onSpawn} />
	<div id="language_picker">
		<LanguageMenu state={$state}/>
	</div>
	<HelpLinks {onCopyInvitationLink}/>
{:else if $state.status.playing}
	<Chat bind:this={chatRef} state={$state} {onMutePlayer} {onReportAbuse} {onSendChat}/>
	<Hint type={$state.status.playing.type}/>
	<Leaderboard state={$state} footer={$state.playerCount ? $t('panel.online.label').replace('{players}', $state.playerCount) : null}/>
    <ShipControls bind:this={shipRef} state={$state} bind:active bind:altitudeTarget bind:selection={armamentSelection}/>
	<ShipStatus state={$state}/>
	<Sidebar onZoom={client.zoom} {onCopyInvitationLink}/>
	<TeamsOverlay state={$state} {onAcceptJoinTeam} {onCreateTeam} {onKickFromTeam} {onLeaveTeam} {onRejectJoinTeam} {onRequestJoinTeam}/>
	{#if canUpgrade($state.status.playing.type, $state.score)}
		<UpgradeMenu
			score={$state.score}
			type={$state.status.playing.type}
			restrictions={$state.restrictions}
			{onUpgrade}
		/>
	{:else}
		<Instructions {touch} {instructBasics} {instructZoom}/>
	{/if}
{:else if $state.status.respawning}
	<XButton on:click={() => client && client.event('OverrideRespawn')}/>
	<RespawnMenu respawnLevel={$state.status.respawning.respawnLevel} state={$state} {onSpawn}/>
	<Sidebar onZoom={client.zoom} {onCopyInvitationLink}/>
	<HelpLinks {onCopyInvitationLink}/>
{:else}
	<WarningPanel message="Invalid state {JSON.stringify($state)}"/>
{/if}

<ContextMenu/>
<Router routes={{
		'/help': HelpDialog,
		'/about': AboutDialog,
		'/settings': wrap({
			component: SettingsDialog,
			props: {
				state
			}
		}),
		'/privacy': PrivacyDialog,
		'/terms': TermsDialog,
		'/ships': ShipsDialog,
		'/levels': LevelsDialog,
		'/changelog': Changelog}}></Router>

<svelte:window
	bind:innerWidth bind:innerHeight
	on:contextmenu={e => e.preventDefault()}
	on:keydown={onKey} on:keyup={onKey}
	on:blur={onChangeWindowFocused}
	on:focus={onChangeWindowFocused}
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

	#language_picker {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
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
