<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import entityData from '../data/entities.json';
	import Link, {outboundEnabled} from '../lib/Link.svelte';
	import Page from '../lib/Page.svelte';
	import t from '../lib/translation.js';

	let shipTypeCount = 0;
	let shipLevelMax = 0;
	let weaponSubTypeCount = 0;

	const weaponSubTypes = {};
	for (const entityType of Object.keys(entityData)) {
		const data = entityData[entityType];
		switch (data.kind) {
			case 'boat':
				shipTypeCount++;
				shipLevelMax = Math.max(shipLevelMax, data.level);
				break;
			case 'weapon':
				weaponSubTypes[data.subkind] = true;
				break;
		}
	}
	weaponSubTypeCount = Object.keys(weaponSubTypes).length;
</script>

<Page>
	<h1>{$t('page.about.title')}</h1>

	<h2>Description</h2>

	<p>Mk48.io is an online multiplayer ship combat game created by Softbear
	Studios. The goal is to level up your ship by collecting crates and sinking
	other ships. There are <a href='/ships'>{shipTypeCount} ships</a> and {weaponSubTypeCount} weapon types to chose from, spread
	over {shipLevelMax} progressively more powerful levels.</p>

	<p>To learn more about the game, visit the <a href='/help'>Help page</a>
	You can also view the <a href='/changelog'>Changelog page</a> to see what changed recently.</p>

	<p>You can also <Link href='https://discord.gg/YMheuFQWTX'>join the Discord server</Link>!</p>

	<h2>Technical details</h2>

	<p>The game's source code and assets are <Link href="https://github.com/SoftbearStudios/mk48">open source</Link>.</p>
	<ul>
		<li>The <Link href="https://github.com/SoftbearStudios/mk48/tree/main/server">server</Link> is written in <Link href="https://golang.org/">Go</Link>.</li>
		<li>The <Link href="https://github.com/SoftbearStudios/mk48/tree/main/client">client</Link> is written in the <Link href="https://kit.svelte.dev/">SvelteKit</Link> JavaScript framework, and
		uses <Link href="https://www.pixijs.com/">PIXI.js</Link> for the 2D graphics.</li>
		<li>The assets were modeled and rendered in <Link href="https://www.blender.org/">Blender</Link>.</li>
		<li>You can contribute to the translations at <Link href="https://crowdl.io/mk48/entries">crowdl.io</Link>.</li>
	</ul>

	{#if $outboundEnabled}
		<h2>Contact Us</h2>

		<p>If you have any feedback to share, business inquiries, or any other
		concern, please contact us by email at
		<a href="mailto:finnbearone@gmail.com">finnbearone@gmail.com</a>.</p>
	{/if}
</Page>

<style>
	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
