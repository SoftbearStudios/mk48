<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from './Section.svelte';
	import t from './translation.js';

	export let state;

	const minNameLength = 1;
	const maxNameLength = 6;

	export let onCreateTeam;
	export let onRequestJoinTeam;
	export let onAcceptJoinTeam;
	export let onRejectJoinTeam;
	export let onKickFromTeam;
	export let onLeaveTeam;

	let newTeamName = '';

	function createTeam() {
		onCreateTeam(newTeamName);
		newTeamName = '';
	}

	async function copyInvite() {
		const inviteLink = `${location.host.startsWith('localhost') ? 'http://localhost:3000' : 'https://mk48.io'}/#${teamInvite}`
		await navigator.clipboard.writeText(inviteLink);
	}

	$: myTeamFull = state.teamMembers && state.teamMembers.length >= 6;
</script>

<Section name={state.teamName || $t('panel.team.label')} emblem={state.teamJoinRequests ? state.teamJoinRequests.length : null}>
	{#if state.teamName}
		<table>
			{#if state.teamMembers}
				{#each state.teamMembers as {playerId, name, captain}, i}
					<tr>
						<td class='name' class:owner={captain}>{name}</td>
						{#if state.teamCaptain}
							<td class='hidden'><button class:hidden={captain}>✔</button></td>
							<td><button class:hidden={captain} on:click={() => onKickFromTeam(playerId)} title={$t('panel.team.action.kick.label')}>✘</button></td>
						{/if}
					</tr>
				{/each}
			{/if}
			{#if state.teamJoinRequests}
				{#each state.teamJoinRequests as {playerId, name}}
					<tr>
						<td class='name pending'>{name}</td>
						<td><button class:disabled={myTeamFull} on:click={() => onAcceptJoinTeam(playerId)} title={$t('panel.team.action.accept.label')}>✔</button></td>
						<td><button on:click={() => onRejectJoinTeam(playerId)} title={$t('panel.team.action.deny.label')}>✘</button></td>
					</tr>
				{/each}
			{/if}
		</table>
		<button on:click={onLeaveTeam}>{$t('panel.team.action.leave.label')}</button>
		{#if state.teamInvite}
			<button on:click={copyInvite} disabled={myTeamFull} title={myTeamFull ? 'Team full' : 'Give this link to players who are not yet in game, to allow them to directly join your team'}>{$t('panel.team.action.invite.label')}</button>
		{/if}
	{:else}
		<table>
			{#if state.teams}
				{#each state.teams as {teamId, name, joining}}
					<tr>
						<td class='name'>{name}</td>
						<td>
							<button class:hidden={joining} on:click={() => onRequestJoinTeam(teamId)}>{$t('panel.team.action.request.label')}</button>
						</td>
					</tr>
				{/each}
			{/if}
			<tr>
				<td class='name'><input type='text' placeholder={$t('panel.team.action.name.label')} maxLength={maxNameLength} bind:value={newTeamName}/></td>
				<td><button disabled={newTeamName.length < minNameLength || newTeamName.length > maxNameLength} on:click={createTeam}>{$t('panel.team.action.create.label')}</button></td>
			</tr>
		</table>
	{/if}
</Section>

<style>
	table {
		color: white;
		width: 100%;
	}

	tr {
		margin-top: 0.25em;
		margin-bottom: 0.25em;
	}

	td.name {
		font-weight: bold;
	}

	td.owner {
		text-decoration: underline;
	}

	td.name.pending {
		filter: brightness(0.7);
	}

	input {
		width: 9em;
	}

	button {
		background-color: transparent;
		border: 0;
		width: min-content;
		padding: 0.1em 0.5em;
	}

	button:hover:not(.disabled) {
		background-color: #00000025;
	}

	button.disabled {
		opacity: 0.5;
	}

	.hidden {
		visibility: hidden;
	}
</style>
