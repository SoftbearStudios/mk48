<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Section from '../component/Section.svelte';
	import ShipMenu from '../component/ShipMenu.svelte';
	import t from '../util/translation.js';
	import {cinematic, upgradeShown} from '../util/settings.js';
	import {nextLevel, scoreToLevel, BOAT_LEVEL_MAX} from '../util/warship.js';

	export let score;
	export let type;
	export let restrictions;
	export let onUpgrade;

    let level;

	$: maxLevel = Math.min(scoreToLevel(score), BOAT_LEVEL_MAX);
	$: minLevel = nextLevel(type);
</script>

<div class='upgrade_menu' class:cinematic={$cinematic}>
	<ShipMenu bind:level={level} maxLevel={maxLevel} minLevel={minLevel} name={($t('panel.upgrade.label.ready')).replace("{level}", level)} bind:open={$upgradeShown} onSelectShip={onUpgrade} restrictions={restrictions} type={type}/>
</div>

<style>
	div.cinematic:not(:hover) {
		opacity: 0;
	}

	div.upgrade_menu {
		left: 50%;
		max-width: 45%;
		min-width: 15%;
		padding-top: 1rem;
		position: absolute;
		transform: translate(-50%, 0);
		width: min-content;
	}
</style>
