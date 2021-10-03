<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import IconButton from './IconButton.svelte';
	import Invite from "svelte-bootstrap-icons/lib/PersonPlus";
	import VolumeEmpty from "svelte-bootstrap-icons/lib/VolumeMute";
	import VolumeDownFill from "svelte-bootstrap-icons/lib/VolumeDownFill";
	import VolumeUpFill from "svelte-bootstrap-icons/lib/VolumeUpFill";
	import ZoomIn from "svelte-bootstrap-icons/lib/ZoomIn";
	import ZoomOut from "svelte-bootstrap-icons/lib/ZoomOut";
	import t from './translation.js';
	import {volume} from '../util/settings.js';

	export let onZoom;
	export let onCopyInvitationLink;
</script>

<div>
	<IconButton tooltip={$t('panel.team.action.invite.label')} onChange={onCopyInvitationLink} icons={[Invite]}/>
	<br/>
	<IconButton tooltip={$t('panel.sidebar.action.zoomIn.hint')} onChange={() => onZoom(-8)} icons={[ZoomIn]}/>
	<br/>
	<IconButton tooltip={$t('panel.sidebar.action.zoomOut.hint')} onChange={() => onZoom(8)} icons={[ZoomOut]}/>
	<br/>
	<IconButton tooltip={$t('setting.volume.label')} value={volume.mapToInteger($volume, 3)} onChange={value => volume.setFromInteger(value, 3)} icons={[VolumeEmpty, VolumeDownFill, VolumeUpFill]}/>
</div>

<style>
	div {
		position: absolute;
		top: 45%;
		height: 10%;
		padding: 0.5em;
		right: 0;
		user-select: none;
	}
</style>
