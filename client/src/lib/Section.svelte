<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {slide} from 'svelte/transition';

	const _DOWN_ARROW = '\u25BC';
	const _RIGHT_ARROW = '\u25BA';

	export let name = '';

	export let open = true;

	function toggleOpen() {
		open = !open;
	}
</script>

<div class=container>
	<h2 on:click={toggleOpen}>
		<span>{open ? _DOWN_ARROW : _RIGHT_ARROW}</span>
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
		user-select: none;
	}

	span {
		font-size: 0.9em;
	}
</style>
