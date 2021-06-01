<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {slide} from 'svelte/transition';

	const _DOWN_ARROW = '\u25BC';
	const _RIGHT_ARROW = '\u25BA';

	export let name = '';
	export let emblem = null;

	export let open = true;

	function toggleOpen() {
		open = !open;
	}
</script>

<div class=container>
	<h2 on:click={toggleOpen}>
		<span>{open ? _DOWN_ARROW : _RIGHT_ARROW}</span>
		{#if !open && emblem}
			<div class='emblem'>{emblem}</div>
		{/if}
		{name}
	</h2>
	{#if open}
		<div transition:slide="{{delay: 100}}">
			<slot/>
		</div>
	{/if}
</div>

<style>
	h2 {
		cursor: pointer;
		font-weight: bold;
		margin: 0px;
		margin-right: 1em;
		user-select: none;
	}

	div.emblem {
		position: absolute;
		right: -0.4em;
		top: -0.4em;
		background-color: #00bfff;
		border-radius: 50%;
		width: 1.15em;
		font-size: 1.25em;
		font-weight: bold;
		height: 1.15em;
		text-align: center;
	}

	span {
		font-size: 0.9em;
	}
</style>
