<script context='module'>
	import {writable} from 'svelte/store';

	// Whether outbound links are enabled
	export const outboundEnabled = writable(true);
</script>

<script>
	export let href = 'javascript:void(0)';
	export let onClick = null;
	export let newTab = false;
	$: target = (newTab || href.startsWith('http')) && $outboundEnabled ? '_blank' : null;

	function click() {
		onClick && onClick();
	}
</script>

{#if !href.startsWith('http') || $outboundEnabled}
	<a {href} {target} on:click={click} rel='noopener'>
		<slot/>
	</a>
{:else}
	<slot/>
{/if}

<style>
	a, p {
		color: white;
		pointer-events: all;
	}
</style>
