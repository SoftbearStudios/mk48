<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Link, {outboundEnabled} from '../component/Link.svelte';
	import Dialog from './Dialog.svelte';
	import t from '../util/translation.js';
	import {BOAT_TYPE_COUNT, BOAT_LEVEL_MAX, WEAPON_SUB_KIND_COUNT} from '../util/warship.js';
</script>

<Dialog title={$t('page.about.title')}>
	<h2>Description</h2>

	<p>Mk48.io is an online multiplayer ship combat game created by Softbear
	Studios. The goal is to level up your ship by collecting crates and sinking
	other ships. There are <a href='#/ships'>{BOAT_TYPE_COUNT} ships</a> and {WEAPON_SUB_KIND_COUNT} weapon types to chose from, spread
	over <a href='#/levels'>{BOAT_LEVEL_MAX} progressively more powerful levels</a>.</p>

	<p>To learn more about the game, visit the <a href='#/help'>Help page</a>
	You can also view the <a href='#/changelog'>Changelog page</a> to see what changed recently.</p>

	<p>You can also <Link href='https://discord.gg/YMheuFQWTX'>join the Discord server</Link>!</p>

	<h2>Technical details</h2>

	<p>The game's source code and assets are <Link href="https://github.com/SoftbearStudios/mk48">open source</Link>.</p>
	<ul>
		<li>The <Link href="https://github.com/SoftbearStudios/mk48/tree/main/client">client</Link> and
			<Link href="https://github.com/SoftbearStudios/mk48/tree/main/server">server</Link> are written in
			<Link href="https://www.rust-lang.org/">Rust</Link>.</li>
		<li>The <Link href="https://github.com/SoftbearStudios/mk48/tree/main/js">client GUI</Link> is written in the <Link href="https://svelte.dev/">Svelte</Link> JavaScript framework.</li>
		<li>The textures were modeled and rendered in <Link href="https://www.blender.org/">Blender</Link>, except for the <Link href="https://opengameart.org/content/simple-seamless-tiles-of-dirt-and-sand-sand2png">sand</Link>, <Link href="https://opengameart.org/content/grass-textureseamless-2d">grass</Link>, and <Link href="https://opengameart.org/content/seamless-snow-texture-0">snow</Link>.</li>
		<li>The title-screen logo was designed by <Link href="https://www.fiverr.com/skydesigner">skydesigner</Link>.</li>
		<li>The sounds are from <Link href='https://github.com/SoftbearStudios/mk48/blob/main/assets/sounds/README.md'>these sources.</Link></li>
		<li>IP Geolocation by <Link href='https://db-ip.com'>DB-IP</Link>.</li>
		<li>You can contribute to the translations at <Link href="https://crowdl.io/mk48/entries">crowdl.io</Link>.</li>
	</ul>

	{#if $outboundEnabled}
		<h2>Contact Us</h2>

		<p>If you have any feedback to share, business inquiries, or any other
		concern, please contact us by email at
		<a href="mailto:contact@mk48.io">contact@mk48.io</a>.</p>
	{/if}
</Dialog>

<style>
	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
