<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import t from '../util/translation.js';
	import Link, {outboundEnabled} from '../component/Link.svelte';
	import IconButton from '../component/IconButton.svelte';
	import Discord from "svelte-bootstrap-icons/lib/Discord";
	import Github from "svelte-bootstrap-icons/lib/Github";
	import {onDestroy} from 'svelte';

	export let onCopyInvitationLink;

	let copiedInvitationLink = false;
	let copyInvitationLinkTimeout = null;
	function copyInvitationLink() {
		if (copyInvitationLinkTimeout) {
			clearTimeout(copyInvitationLinkTimeout);
		}
		onCopyInvitationLink();
		copiedInvitationLink = true;
		copyInvitationLinkTimeout = setTimeout(() => {
			copiedInvitationLink = false;
		}, 5000);
	}

	onDestroy(() => {
		if (copyInvitationLinkTimeout) {
			clearTimeout(copyInvitationLinkTimeout);
		}
	});

	const ICON_SIZE = 2.5;
</script>

<span id="help_links">
	<a href='#/help'>{$t('panel.splash.action.help.label')}</a>
	<a href='#/about'>{$t('panel.splash.action.about.label')}</a>
	<a href='#/privacy'>{$t('panel.splash.action.privacy.label')}</a>
	<a href='#/terms'>{$t('panel.splash.action.terms.label')}</a>
</span>

{#if $outboundEnabled}
	<span id="social_links">
		<IconButton tooltip={"Discord"} onChange={() => window.open('https://discord.gg/YMheuFQWTX', '_blank')} icons={[Discord]} size={ICON_SIZE}/>
		<IconButton tooltip={"GitHub"} onChange={() => window.open('https://github.com/SoftbearStudios/mk48', '_blank')} icons={[Github]} size={ICON_SIZE}/>
	</span>
{/if}

{#if onCopyInvitationLink}
	<span id="invite_code" class:disabled={copiedInvitationLink}>
		<Link onClick={copyInvitationLink}>{copiedInvitationLink ? "Copied!" : $t('panel.team.action.invite.label')}</Link>
	</span>
{/if}

<style>
	a {
		color: #FFFC;
		font-size: 1.3rem;
		margin-left: 0.25rem;
		margin-right: 0.25rem;
		padding-top: 3rem;
		white-space: nowrap;
	}

	#help_links {
		bottom: 0.5rem;
		display: flex;
		font-size: 1rem;
		gap: 1rem;
		justify-content: center;
		left: 0;
		position: absolute;
		right: 0;
	}

	#social_links {
		bottom: 0.75rem;
		display: flex;
		font-size: 1rem;
		gap: 1rem;
		justify-content: right;
		position: absolute;
		right: 1rem;
	}

	#invite_code {
		font-size: 1.3rem;
		bottom: 0.5rem;
		left: 0.7rem;
		position: absolute;
	}

	.disabled {
		filter: opacity(0.6);
	}
</style>
