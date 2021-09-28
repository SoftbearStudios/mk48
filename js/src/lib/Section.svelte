<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {slide} from 'svelte/transition';

	export let name = '';
	export let headerAlign = 'left';
	export let emblem = null;

	export let open = true;

	function toggleOpen() {
		open = !open;
	}
</script>

<div class=container>
	<h2 on:click={toggleOpen} style={`text-align: ${headerAlign};`}>
		{name}
		{#if emblem && !open}
			<div class='emblem'>{emblem}</div>
		{/if}
	</h2>
	{#if open}
		<div transition:slide="{{delay: 100}}">
			<slot/>
		</div>
	{/if}
</div>

<style>
	h2 {
		color: white;
		cursor: pointer;
		font-weight: bold;
		margin: 0;
		user-select: none;
		text-align: center;
		transition: filter 0.1s;
	}

	h2:hover {
		filter: opacity(0.85);
	}

	div.container {
		position: relative;
	}

	div.emblem {
		background-color: #00bfff;
		border-radius: 50%;
		width: 1em;
		font-size: 1em;
		font-weight: bold;
		height: 1em;
		text-align: center;
		display: inline-block;
		margin-left: 0.2em;
	}
</style>
