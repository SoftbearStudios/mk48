<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import spriteSheet from "../data/sprites_css.json";
	import {hasWebP} from "../util/compatibility.js";
	export let name;
	export let title;
	export let consumed;
	$: sprite = spriteSheet.sprites[name];
</script>

<div {title} class:consumed class:webp={hasWebP()} on:click style={`background-position: -${sprite.x}px -${sprite.y}px; width: ${sprite.width}px; height: ${sprite.height}px;`}/>

<style>
	div {
		background-image: url("/sprites_css.png");
		display: inline-block;
	}

	div.webp {
		background-image: url("/sprites_css.webp");
	}

	div.consumed {
		opacity: 0.6;
	}
</style>
