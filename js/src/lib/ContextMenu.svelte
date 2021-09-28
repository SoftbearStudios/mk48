<script context='module'>
	import {get, writable} from 'svelte/store';

	let contextMenu = writable(null);

	export function showContextMenu(event, options) {
		event.preventDefault();
		event.stopPropagation();
		contextMenu.set({
			x: event.pageX,
			y: event.pageY,
			options
		});
	}

	function onHide(event) {
		if (get(contextMenu)) {
			event.preventDefault();
			contextMenu.set(null);
		}
	}

	function onClick(event, callback) {
		onHide(event);
		callback();
	}
</script>

<svelte:body on:contextmenu={onHide}/>

{#if $contextMenu}
	<div style={`left: ${$contextMenu.x}px; top: ${$contextMenu.y}px;`}>
		{#each Object.entries($contextMenu.options) as [name, callback]}
			<button on:click={event => onClick(event, callback)}>{name}</button>
		{/each}
	</div>
{/if}

<style>
	button {
		color: white;
		background-color: #444444aa;
		border: 0;
		border-radius: 0;
		outline: 0;
		margin: 0;
		padding: 5px;
	}

	button:hover {
		filter: brightness(1.1);
	}

	button:hover:active {
		filter: brightness(1.05);
	}

	div {
		background-color: #444444aa;
		min-width: 100px;
		position: absolute;
	}
</style>
