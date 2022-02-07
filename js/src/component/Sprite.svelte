<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import spriteSheet from "../data/sprites_css.json";
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
    <div class="container" on:click style={`width: ${sprite.width}px; height: ${sprite.height}px;`}>
        <div {title} class="inner" class:consumed class:selectable={true} style={`background-position: -${sprite.x}px -${sprite.y}px; width: ${sprite.width}px; height: ${sprite.height}px;`}></div>
        {#if icon}
            <div class="icon">
                <IconButton tooltip={iconTitle} icons={[icon]} size={1.5} onChange={onIconClick}/>
            </div>
        {/if}
    </div>
{/if}

<style>
    div.container {
        position: relative;
        text-align: center;
        display: inline-block;
    }

	div.inner {
		background-image: url("/sprites_css.png");
        position: absolute;
	}

    div.icon {
        margin-top: 0.25em;
        opacity: 0.8;
    }

	div.consumed {
		opacity: 0.6;
	}

	div.selectable {
		cursor: pointer;
	}
</style>
