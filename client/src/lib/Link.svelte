<script>
	export let href = 'javascript:void(0)';
	export let onClick = null;
	export let newTab = false;
	$: target = newTab || href.startsWith('http') ? '_blank' : null;

	function click() {
		onClick && onClick();
	}
</script>

<a {href} {target} on:click={click} rel='noopener'>
	<slot/>
</a>

<style>
	a {
		color: white;
		pointer-events: all;
	}
</style>
