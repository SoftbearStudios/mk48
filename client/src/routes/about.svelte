<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';

	let shipTypeCount = 0;
	let shipLevelMax = 0;
	let weaponSubTypeCount = 0;

	const weaponSubTypes = {};
	for (const entityType of Object.keys(entityData)) {
		const data = entityData[entityType];
		switch (data.type) {
			case 'boat':
				shipTypeCount++;
				shipLevelMax = Math.max(shipLevelMax, data.level);
				break;
			case 'weapon':
				weaponSubTypes[data.subtype] = true;
				break;
		}
	}
	weaponSubTypeCount = Object.keys(weaponSubTypes).length;
</script>

<div>
	<h1>About Mk48.io</h1>

	<h2>Description</h2>

	<p>Mk48.io is an online multiplayer ship combat game created by Softbear
	Studios. The goal is to level up your ship by collecting crates and sinking
	other ships. There are {shipTypeCount} ships and {weaponSubTypeCount} weapon types to chose from, spread
	over {shipLevelMax} progressively more powerful levels.</p>

	<p>To learn more about the game, visit the <a href='/help'>Help page</a>.

	<h2>Technical details</h2>

	<p>The game's server is written in Go.</p>
	<p>The game's client is written in the SvelteKit JavaScript framework, and
	uses PIXI.js for the 2D graphics.</p>
	<p>The game assets were modeled and rendered in Blender.</p>

	<h2>Contact Us</h2>

	<p>If you have any feedback to share, business inquiries, or any other
	concern, please contact us by email at
	<a href="mailto:finnbearone@gmail.com">finnbearone@gmail.com</a>.</p>
</div>

<style>
	div {
		background-color: #2c3e50;
		color: white;
		font-family: sans-serif;
		font-size: 20px;
		overflow-y: scroll;
		padding: 10px;
		position: absolute;
		inset: 0;
	}

	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
