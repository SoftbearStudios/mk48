<script context=module>
	const _BAR_COLOR = '#0084b1';
	const _BACKGROUND_COLOR = '#3f3333';
</script>

<script>
	export let value;
	export let color = _BAR_COLOR;
	$: percentage = Math.round(value * 100);

</script>

<div class=meter style={`background: linear-gradient(90deg, ${color} 0%, ${color} 1%, ${_BACKGROUND_COLOR} 1%, ${_BACKGROUND_COLOR} 100%); background-size: ${Math.max(percentage, 1) * 100}%;`}>
	<slot/>
</div>

<style>
	div.meter {
		background-color: #bbbbbb;
		border: 1px solid #dddddd;
		border-radius: 0.5em;
		box-sizing: border-box;
		color: white;
		font-weight: bold;
		height: min-content;
		min-height: 1.1em;
		overflow: hidden;
		padding: 0.2em;
		text-align: center;
		transition: background-size 0.5s;
		user-select: none;
		width: 100%;
	}
</style>
