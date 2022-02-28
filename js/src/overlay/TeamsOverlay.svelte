<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from '../component/Section.svelte';
	import t from '../util/translation.js';
	import {cinematic, teamsShown} from '../util/settings.js';

	export let onAcceptJoinTeam;
	export let onCreateTeam;
	export let onKickFromTeam;
	export let onLeaveTeam;
	export let onRejectJoinTeam;
	export let onRequestJoinTeam;

	export let state;

	const MIN_NAME_LENGTH = 1;
	const MAX_NAME_LENGTH = 6;

	let newTeamName = '';

	async function copyInvite() {
		const inviteLink = `${location.host.startsWith('localhost') ? 'http://localhost:3000' : 'https://mk48.io'}/#${teamInvite}`
		await navigator.clipboard.writeText(inviteLink);
	}

	function createTeam() {
		if (newTeamName.length == 0) {
			return;
		}
		onCreateTeam(newTeamName);
		newTeamName = '';
	}
</script>

<div id="teams_overlay" class:cinematic={$cinematic}>
	<Section name={state.teamName || $t('panel.team.label')} emblem={state.teamJoinRequests ? state.teamJoinRequests.length : null} bind:open={$teamsShown}>
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
							<td><button class:disabled={state.teamFull} on:click={() => onAcceptJoinTeam(playerId)} title={$t('panel.team.action.accept.label')}>✔</button></td>
							<td><button on:click={() => onRejectJoinTeam(playerId)} title={$t('panel.team.action.deny.label')}>✘</button></td>
						</tr>
					{/each}
				{/if}
			</table>
			<button on:click={onLeaveTeam}>{$t('panel.team.action.leave.label')}</button>
			{#if state.teamInvite}
				<button on:click={copyInvite} disabled={state.teamFull} title={state.teamFull ? 'Team full' : 'Give this link to players who are not yet in game, to allow them to directly join your team'}>{$t('panel.team.action.invite.label')}</button>
			{/if}
		{:else}
			<form on:submit|preventDefault={createTeam}>
				<table>
					{#if state.teams}
						{#each state.teams as {teamId, name, joining, closed}}
							<tr>
								<td class='name'>{name}</td>
								<td>
									<button class:hidden={joining || closed} on:click={() => onRequestJoinTeam(teamId)}>{$t('panel.team.action.request.label')}</button>
								</td>
							</tr>
						{/each}
					{/if}
					<tr>
						<td class='name'><input type='text' placeholder={$t('panel.team.action.name.label')} maxLength={MAX_NAME_LENGTH} bind:value={newTeamName}/></td>
						<td><button disabled={newTeamName.length < MIN_NAME_LENGTH || newTeamName.length > MAX_NAME_LENGTH}>{$t('panel.team.action.create.label')}</button></td>
					</tr>
				</table>
			</form>
		{/if}
	</Section>
</div>

<style>
	#teams_overlay {
		left: 0;
		max-width: 25%;
		padding-left: 1rem;
		padding-top: 1rem;
		position: absolute;
		top: 0;
	}

	button {
		background-color: transparent;
		border: 0;
		width: min-content;
		padding: 0.1em 0.5em;
	}

	button.disabled {
		opacity: 0.5;
	}

	button:hover:not(.disabled) {
		background-color: #00000025;
	}

	div.cinematic:not(:hover) {
		opacity: 0;
	}

	input {
		width: 9em;
	}

	.hidden {
		visibility: hidden;
	}

	table {
		color: white;
		width: 100%;
	}

	td.name {
		font-weight: bold;
	}

	td.name.pending {
		filter: brightness(0.7);
	}

	td.owner {
		text-decoration: underline;
	}

	tr {
		margin-top: 0.25em;
		margin-bottom: 0.25em;
	}
</style>
