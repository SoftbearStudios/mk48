<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {slide} from 'svelte/transition';
	import LeftArrowIcon from "svelte-bootstrap-icons/lib/CaretLeftSquareFill";
	import RightArrowIcon from "svelte-bootstrap-icons/lib/CaretRightSquareFill";

	export let disableLeftArrow = false;
	export let disableRightArrow = false;
	export let headerAlign = 'left';
	export let emblem = null;
	export let name = '';
	export let open = true;
	export let onClick = null;
	export let closable = true;
	export let onLeftArrow = null;
	export let onRightArrow = null;

	function toggleOpen() {
		if (closable && (onClick === null || onClick())) {
			open = !open;
		}
	}
</script>

<h2 class:closable on:click={toggleOpen} style={`text-align: ${headerAlign};`}>
	{#if onLeftArrow && open}
		<span class:disable={disableLeftArrow} on:click={e => {e.stopPropagation(); if (!disableLeftArrow) onLeftArrow();}}>
			<LeftArrowIcon/> 
		</span>
	{/if}
	{name}
	{#if emblem && !open}
		<div class='emblem'>{emblem}</div>
	{/if}
	{#if onRightArrow && open}
		<span class:disable={disableRightArrow} on:click={e => {e.stopPropagation(); if (!disableRightArrow) onRightArrow();}}>
			<RightArrowIcon/> 
		</span>
	{/if}
</h2>
{#if open}
	<div class:prevent={!closable} transition:slide="{{delay: 100}}">
		<slot/>
	</div>
{/if}

<style>
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

	div.prevent {
		animation: none !important;
	}

	h2 {
		color: white;
		font-weight: bold;
		margin: 0;
		user-select: none;
		text-align: center;
		transition: filter 0.1s;
	}

	h2.closable {
		cursor: pointer;
	}

	h2.closable:hover {
		filter: opacity(0.85);
	}

	span.disable {
		opacity: 0;
	}

	@media(min-width: 1000px) {
		h2 {
			white-space: nowrap;
		}
	}
</style>
