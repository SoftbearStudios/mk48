<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import Section from './Section.svelte';
	import {get} from 'svelte/store';
	import {entityID, playerID as socketPlayerID, send, teamInvite, teamMembers, teamJoinRequests} from './socket.js';

	function getTeams(contacts) {
		const teams = {};
		for (const contact of Object.values(contacts)) {
			if (contact.team) {
				teams[contact.team] = true;
			}
		}
		return Object.keys(teams).slice(0, 5);
	}

	async function copyInvite() {
		const inviteLink = `${location.host.startsWith('localhost') ? 'http://localhost:3000' : 'https://mk48.io'}/#${get(teamInvite)}`
		await navigator.clipboard.writeText(inviteLink);
	}
</script>

<script>
	export let contacts;

	let newTeamName = '';

	const minNameLength = 2;
	const maxNameLength = 6;

	function createTeam() {
		send('createTeam', {name: newTeamName});
		newTeamName = '';
	}

	$: myTeamID = contacts[$entityID] ? contacts[$entityID].team : null;
	$: isOwner = $teamMembers && $teamMembers[0] && $socketPlayerID == $teamMembers[0].playerID;
</script>

<div>
	<Section name={myTeamID || 'Fleet'} open={false}>
		{#if myTeamID}
			<table>
				{#if $teamMembers}
					{#each $teamMembers as {playerID, name}, i}
						<tr>
							<td class='name'>{name}</td>
							{#if isOwner && i > 0}
								<td><button on:click={() => send('removeFromTeam', {playerID})}>{i == 0 ? 'Leave' : 'Kick'}</button></td>
							{/if}
						</tr>
					{/each}
				{/if}
				{#if $teamJoinRequests}
					{#each $teamJoinRequests as {playerID, name}}
						<tr>
							<td class='name'>{name}</td>
							<td><button on:click={() => send('addToTeam', {playerID})}>Accept</button></td>
						</tr>
					{/each}
				{/if}
			</table>
			{#if $teamInvite}
				<button on:click={copyInvite}>Copy Invite</button>
			{/if}
			<button on:click={() => send('removeFromTeam')}>Leave</button>
		{:else}
			<table>
				{#each getTeams(contacts) as teamID}
					<tr>
						<td class='name'>{teamID}</td>
						<td><button on:click={() => send('addToTeam', {teamID})}>Request Join</button></td>
					</tr>
				{/each}
				<tr>
					<td class='name'><input type='text' placeholder='Fleet name' maxLength={maxNameLength} bind:value={newTeamName}/></td>
					<td><button disabled={newTeamName.length < minNameLength || newTeamName.length > maxNameLength} on:click={createTeam}>Create</button></td>
				</tr>
			</table>
		{/if}
	</Section>
</div>

<style>
	div {
		bottom: 45%;
		background-color: #00000040;
		left: 0;
		margin: 10px;
		min-width: 150px;
		padding: 10px;
		position: absolute;
	}

	h2 {
		margin-bottom: 10px;
		margin-top: 0px;
	}

	table {
		width: 100%;
	}

	a {
		color: white;
	}
</style>
