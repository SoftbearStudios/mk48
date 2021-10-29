<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import spriteSheet from "../data/sprites_css.json";
	import {hasWebP} from "../util/compatibility.js";
	import IconButton from './IconButton.svelte';

	export let name;
	export let title = '';
	export let consumed = false;
	export let icon = null;
	export let iconTitle = null;
	export let onIconClick = null;
	$: sprite = spriteSheet.sprites[name];
</script>

{#if sprite}
    <div {title} class:consumed class:webp={hasWebP()} on:click style={`background-position: -${sprite.x}px -${sprite.y}px; width: ${sprite.width}px; height: ${sprite.height}px;`}>
        {#if icon}
            <IconButton tooltip={iconTitle} icons={[icon]} size={1.5} onChange={onIconClick}/>
        {/if}
    </div>
{/if}

<style>
	div {
		background-image: url("/sprites_css.png");
		display: inline-block;
		text-align: center;
	}

	div.webp {
		background-image: url("/sprites_css.webp");
	}

	div.consumed {
		opacity: 0.6;
	}
</style>
