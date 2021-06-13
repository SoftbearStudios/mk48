<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script context='module'>
	import Section from './Section.svelte';
	import t from './translation.js';
	import {get} from 'svelte/store';
	import {entityID, playerID as socketPlayerID, send, teamInvite, teamMembers, teamJoinRequests} from './socket.js';

	function getTeams(contacts) {
		const teams = {};
		for (const contact of Object.values(contacts)) {
			if (contact.team && !contact.teamFull) {
				teams[contact.team] = true;
			}
		}
		return Object.keys(teams).slice(0, 5).sort();
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

	$: me = contacts[$entityID];
	$: myTeamID = me ? me.team : null;
	$: myTeamFull = me ? me.teamFull : false;
	$: isOwner = $teamMembers && $teamMembers[0] && $socketPlayerID == $teamMembers[0].playerID;
</script>

<Section name={myTeamID || $t('panel.team.label')} emblem={$teamJoinRequests ? ($teamJoinRequests).length : null} open={false}>
	{#if myTeamID}
		<table>
			{#if $teamMembers}
				{#each $teamMembers as {playerID, name}, i}
					<tr>
						<td class='name'>{name}</td>
						{#if isOwner && i > 0}
							<td><button on:click={() => send('removeFromTeam', {playerID})}>{$t('panel.team.action.kick.label')}</button></td>
						{/if}
					</tr>
				{/each}
			{/if}
			{#if $teamJoinRequests}
				{#each $teamJoinRequests as {playerID, name}}
					<tr>
						<td class='name'>{name}</td>
						<td><button on:click={() => send('addToTeam', {playerID})}>{$t('panel.team.action.accept.label')}</button></td>
					</tr>
				{/each}
			{/if}
		</table>
		{#if $teamInvite}
			<button on:click={copyInvite} disabled={myTeamFull} title={myTeamFull ? 'Team full' : 'Give this link to players who are not yet in game, to allow them to directly join your team'}>{$t('panel.team.action.invite.label')}</button>
		{/if}
		<button on:click={() => send('removeFromTeam')}>{$t('panel.team.action.leave.label')}</button>
	{:else}
		<table>
			{#each getTeams(contacts) as teamID}
				<tr>
					<td class='name'>{teamID}</td>
					<td><button on:click={() => send('addToTeam', {teamID})}>{$t('panel.team.action.request.label')}</button></td>
				</tr>
			{/each}
			<tr>
				<td class='name'><input type='text' placeholder={$t('panel.team.action.name.label')} maxLength={maxNameLength} bind:value={newTeamName}/></td>
				<td><button disabled={newTeamName.length < minNameLength || newTeamName.length > maxNameLength} on:click={createTeam}>{$t('panel.team.action.create.label')}</button></td>
			</tr>
		</table>
	{/if}
</Section>

<style>
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
