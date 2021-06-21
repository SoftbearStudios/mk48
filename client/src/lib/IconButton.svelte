<script>
	import DefaultOff from "svelte-bootstrap-icons/lib/Square";
	import DefaultOn from "svelte-bootstrap-icons/lib/SquareFill";

	export let tooltip = null;
	export let value = null;
	export let onChange;
	export let icons = [DefaultOff, DefaultOn];
	export let size = 2;
	$: sizeEms = `${size}em`

	function getIcon(value) {
		switch (icons.length) {
			case 1:
				return icons[0];
			case 2:
				return icons[value ? 1 : 0];
			default:
				return icons[Math.round(value || 0)];
		}
	}

	function handleClick() {
		switch (icons.length) {
			case 1:
				onChange();
				break;
			case 2:
				value = !value;
				onChange(value);
				break;
			default:
				value = ((value || 0) + 1 + icons.length) % icons.length;
				onChange(value);
				break;
		}
	}
</script>

<span title={tooltip} class:selected={value || icons.length === 1} on:click={handleClick}>
	<svelte:component this={getIcon(value)} width={sizeEms} height={sizeEms}/>
</span>

<style>
	span {
		cursor: pointer;
		filter: brightness(0.9);
	}

	div {
		display: table-row;
	}

	p {
		color: inherit;
		display: table-cell;
		user-select: none;
		vertical-align: middle;
	}
</style>
