<script>
	import IconButton from './IconButton.svelte';
	import VolumeEmpty from "svelte-bootstrap-icons/lib/VolumeMute";
	import VolumeDownFill from "svelte-bootstrap-icons/lib/VolumeDownFill";
	import VolumeUpFill from "svelte-bootstrap-icons/lib/VolumeUpFill";
	import ZoomIn from "svelte-bootstrap-icons/lib/ZoomIn";
	import ZoomOut from "svelte-bootstrap-icons/lib/ZoomOut";
	import t from './translation.js';
	import {volume} from './settings.js';

	export let zoom;
</script>

<div>
	<IconButton tooltip={$t('panel.sidebar.action.zoomIn.hint')} onChange={() => zoom(1)} icons={[ZoomIn]}/>
	<br/>
	<IconButton tooltip={$t('panel.sidebar.action.zoomOut.hint')} onChange={() => zoom(-1)} icons={[ZoomOut]}/>
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
