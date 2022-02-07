<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Dialog from './Dialog.svelte';
	import t from '../util/translation.js';
</script>

<Dialog title={$t('page.terms.title')}>
	<p>The following terms govern your use of the Mk48.io website and game.</p>

	<h2>Allowed Activities</h2>

	<p>You are granted a non-exclusive license to do the following for either
	commercial or non-commercial purposes, provided you adhere to all the terms.</p>

	<ol>
		<li>Playing the game.</li>
		<li>Recording and/or publishing screenshots, videos, or other content involving the game.</li>
		<li>Using the individual game textures or adaptations of them in connection with content involving the game (such as in a video thumbnail).</li>
		<li>Embedding the Mk48.io game website on another website, provided that the game retains its full title ("Mk48.io").</li>
		<li>Linking to the game on another website.</li>
	</ol>

	<h2>Prohibited Activities</h2>

	<p>You are prohibited from engaging in any of the following activities.</p>

	<ol>
		<li>Disclosing any personal information (full name, contact information, etc.), by any means, if you are under 13 years of age.</li>
		<li>Using inappropriate or offensive language for a player nickname, team name, in game chat, or in any comments section corresponding to the game.</li>
		<li>Placing a higher burden on the game's server(s) than two instances of the official game client would (opening more than two connections at a time, sending messages at a higher frequency, or otherwise compromising the integrity of the server(s)).</li>
		<li>Violating any applicable law, or violating the rights or privacy of others.</li>
		<li>Claiming to offer a downloadable version of the game (as one does not exist).</li>
		<li>Attempting to bypass our policy preventing leaderboard score attempts on non-public servers.</li>
	</ol>

	<h2>Liability</h2>

	<p><b>Mk48.io is provided "AS IS". The developers make no warranties, express
	or implied, and hereby disclaim all implied warranties, including any warranty of
	merchantibility and warranty of fitness of fitness for a particular purpose.</b></p>

	<h2>Changes</h2>

	<p>We reserve the right to alter these Terms of Service at any time, without notice.</p>
</Dialog>

<style>
	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
